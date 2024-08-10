#[macro_use]
extern crate rocket;

mod crypto;
mod database;
mod helpers;
mod timestamps;

use database::{
	types::{AccountId, PostId},
	Account, AccountError, Post,
};
use futures::future;
use helpers::{CookieJarHelper, ErrorHelper};
use rocket::{
	form::{Form, FromForm},
	fs::FileServer,
	http::{uri::Origin, Cookie, CookieJar, Status},
	response::{content::RawHtml, Redirect},
};
use rocket_dyn_templates::tera::{Context, Tera};
use serde::Serialize;
use std::sync::LazyLock;

// global constants
static TERA: LazyLock<Tera> = LazyLock::new(|| {
	Tera::new("./templates/**/*.html").expect("Could not load templates from ./templates")
});

// session structs
#[derive(Serialize)]
struct SessionUser {
	id: AccountId,
	handle: Box<str>,
}
impl SessionUser {
	fn try_from_jar(jar: &CookieJar<'_>) -> Option<Self> {
		Some(Self {
			id: jar.get_private("id")?.value().parse::<u32>().ok()?.into(),
			handle: jar.get_private("handle")?.value().into(),
		})
	}
}
#[derive(Serialize)]
struct SessionData {
	user: Option<SessionUser>,
}
impl<'a> From<&CookieJar<'a>> for SessionData {
	fn from(value: &CookieJar<'a>) -> Self {
		Self {
			user: SessionUser::try_from_jar(value),
		}
	}
}

#[derive(FromForm)]
struct AuthInput<'a> {
	username: Option<&'a str>,
	password: Option<&'a str>,
}

#[derive(FromForm)]
struct PostInput {
	body: Option<String>,
	parent_id: Option<PostId>,
}

#[derive(Serialize)]
pub struct BaseTemplatePost {
	pub id: u64,
	pub author_id: u32,
	pub author_name: Box<str>,
	pub author_handle: Box<str>,
	pub body: Box<str>,
	pub create_time: Box<str>,
	pub likes: u64,
	pub liked_by_user: bool,
	pub parent_id: Option<u64>,
}

impl BaseTemplatePost {
	async fn from_post(post: Post, user: &Option<SessionUser>) -> sqlx::Result<Self> {
		let author = Account::find_by_id(post.author_id).await?;
		let author_name = author
			.as_ref()
			.map(|acc| acc.display_name())
			.unwrap_or("Account does not exist")
			.into();
		let author_handle = author
			.map(|a| a.handle)
			.unwrap_or_default()
			.into_boxed_str();

		let liked_by_user = match user {
			Some(u) => post.voted_by(u.id).await.unwrap_or(false),
			None => false,
		};

		Ok(Self {
			id: post.id.0,
			author_id: post.author_id.0,
			author_name,
			author_handle,
			body: post.body,
			create_time: timestamps::format_timestamp(post.create_time).into_boxed_str(),
			likes: post.votes.0,
			liked_by_user,
			parent_id: post.parent_id.0.map(Into::into),
		})
	}
}

#[derive(Serialize)]
pub struct ReplyTemplatePost {
	base: BaseTemplatePost,
	reply: Option<BaseTemplatePost>,
}
impl ReplyTemplatePost {
	async fn from_posts(posts: Vec<Post>, user: &Option<SessionUser>) -> sqlx::Result<Vec<Self>> {
		future::join_all(posts.into_iter().map(|post| async {
			let mut replies = post.get_replies(1).await?;
			Ok(Self {
				base: BaseTemplatePost::from_post(post, user).await?,
				reply: match replies.pop() {
					Some(p) => Some(BaseTemplatePost::from_post(p, user).await?),
					None => None,
				},
			})
		}))
		.await
		// convert from vec<result<template, error>> into result<vec<template>, error>
		.into_iter()
		.collect::<sqlx::Result<Vec<Self>>>()
	}
}

#[launch]
fn rocket() -> _ {
	rocket::build()
		.mount(
			"/",
			routes![
				favicon,
				get_feed,
				get_post,
				create_post,
				delete_post,
				like_post,
				get_user,
				get_login,
				get_register,
				login,
				register,
				signout,
				delete_account
			],
		)
		.mount("/static", FileServer::from("./static"))
}

// favicon
#[get("/favicon.ico")]
fn favicon() -> Redirect {
	Redirect::permanent(uri!("/static/favicon.ico"))
}

// posts

#[get("/")]
async fn get_feed(jar: &CookieJar<'_>) -> Result<RawHtml<String>, Status> {
	let session_data: SessionData = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &session_data.user);

	// inserting posts
	let posts = match Post::get_recent(100).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	let posts = match ReplyTemplatePost::from_posts(posts, &session_data.user).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("feed.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[get("/post/<post_id>")]
async fn get_post(jar: &CookieJar<'_>, post_id: PostId) -> Result<RawHtml<String>, Status> {
	let session_data: SessionData = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &session_data.user);

	// inserting post
	let post = match Post::find_by_id(post_id).await {
		Ok(Some(p)) => p,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};
	// inserting replies
	let replies = match post.get_replies(100).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	let replies = match ReplyTemplatePost::from_posts(replies, &session_data.user).await {
		Ok(r) => r,
		Err(e) => return e.print_and_err(),
	};
	context.insert("replies", &replies);

	let template_post = match BaseTemplatePost::from_post(post, &session_data.user).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	context.insert("current_post", &template_post);

	// rendering the template
	match template_post.parent_id {
		None => match TERA.render("post.html", &context) {
			Ok(s) => Ok(RawHtml(s)),
			Err(e) => e.print_and_err(),
		},
		Some(parent_id) => {
			// inserting parent
			let parent_post = match Post::find_by_id(PostId(parent_id)).await {
				Ok(Some(post)) => {
					match BaseTemplatePost::from_post(post, &session_data.user).await {
						Ok(p) => p,
						Err(e) => return e.print_and_err(),
					}
				}
				Ok(None) => return Err(Status::NotFound),
				Err(e) => return e.print_and_err(),
			};

			context.insert("parent_post", &parent_post);

			match TERA.render("reply_post.html", &context) {
				Ok(s) => Ok(RawHtml(s)),
				Err(e) => e.print_and_err(),
			}
		}
	}
}

#[post("/create_post", data = "<post_input>")]
async fn create_post(jar: &CookieJar<'_>, post_input: Form<PostInput>) -> Result<Redirect, Status> {
	let session_data: SessionData = jar.into();

	let body = match &post_input.body {
		Some(b) => b.as_ref(),
		None => return Err(Status::BadRequest),
	};

	let account = match session_data.user {
		Some(user) => match Account::find_by_id(user.id).await {
			Ok(Some(acc)) => acc,
			Ok(None) => return Err(Status::Unauthorized),
			Err(e) => return e.print_and_err(),
		},
		None => return Err(Status::Unauthorized),
	};

	match account.create_post(body, post_input.parent_id).await {
		Ok(_) => Ok(Redirect::to(match post_input.parent_id {
			Some(id) => format!("/post/{}", id.0),
			None => "/".to_string(),
		})),
		Err(e) => e.print_and_err(),
	}
}

#[get("/delete_post/<post_id>")]
async fn delete_post(jar: &CookieJar<'_>, post_id: PostId) -> Result<Redirect, Status> {
	let session_data: SessionData = jar.into();

	let user = match session_data.user {
		Some(user) => user,
		None => return Err(Status::Unauthorized),
	};
	let post = match Post::find_by_id(post_id).await {
		Ok(Some(post)) => post,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};
	if user.id != post.author_id {
		return Err(Status::Unauthorized);
	}
	match post.delete().await {
		Ok(Some(parent_id)) => Ok(Redirect::to(format!("/post/{}", parent_id.0))),
		Ok(None) => Ok(Redirect::to("/")),
		Err(e) => e.print_and_err(),
	}
}

#[get("/like_post/<post_id>")]
async fn like_post(jar: &CookieJar<'_>, post_id: PostId) -> Status {
	let session_data: SessionData = jar.into();

	let user = match session_data.user {
		Some(user) => user,
		None => return Status::Unauthorized,
	};
	let account = match Account::find_by_id(user.id).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Status::Unauthorized,
		Err(e) => return e.print_and_status(),
	};

	let post = match Post::find_by_id(post_id).await {
		Ok(Some(post)) => post,
		Ok(None) => return Status::NotFound,
		Err(e) => return e.print_and_status(),
	};

	if post.author_id == account.id {
		return Status::Forbidden;
	}

	let liked_by_user = match post.voted_by(account.id).await {
		Ok(v) => v,
		Err(e) => return e.print_and_status(),
	};

	let res = if liked_by_user {
		account.remove_vote(post.id).await
	} else {
		account.add_vote(post.id).await.map(|_| ())
	};
	match res {
		Ok(_) => Status::Ok,
		Err(e) => e.print_and_status(),
	}
}

#[get("/user/<handle>")]
async fn get_user(jar: &CookieJar<'_>, handle: &str) -> Result<RawHtml<String>, Status> {
	let session_data: SessionData = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting handle
	context.insert("handle", handle);

	// inserting user data
	context.insert("user", &session_data.user);

	let account = match Account::find_by_handle(handle).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};

	// inserting posts
	let posts = match account.get_posts(100, false).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	let posts = match ReplyTemplatePost::from_posts(posts, &session_data.user).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("user.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

// accounts

#[get("/login")]
fn get_login(jar: &CookieJar<'_>, origin: &Origin) -> Result<RawHtml<String>, Status> {
	let session_data: SessionData = jar.into();

	// creating template context
	let mut context = Context::new();

	if let Some(q) = origin.query() {
		for pair in q.segments() {
			match pair {
				("err", "password") => {
					context.insert("error", "Incorrect password!");
					break;
				}
				("err", "handle") => {
					context.insert("error", "A user with this handle doesn't exist");
					break;
				}
				(_, _) => (),
			};
		}
	}

	if session_data.user.is_some() {
		return Err(Status::BadRequest);
	}

	// inserting user data
	context.insert("user", &session_data.user);

	// render the template
	match TERA.render("login.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[post("/login", data = "<login_input>")]
async fn login(jar: &CookieJar<'_>, login_input: Form<AuthInput<'_>>) -> Result<Redirect, Status> {
	let (handle, password) = match (login_input.username, login_input.password) {
		(Some(u), Some(p)) => (u, p),
		_ => return Err(Status::BadRequest),
	};

	use AccountError::*;
	match Account::login(handle, password).await {
		Ok(acc) => {
			jar.add_private(Cookie::new("id", acc.id.0.to_string()));
			jar.add_private(Cookie::new("handle", acc.handle));
			Ok(Redirect::to("/"))
		}
		Err(Handle(_)) => Ok(Redirect::to("/login?err=handle")),
		Err(Password(_)) => Ok(Redirect::to("/login?err=password")),
		Err(Sqlx(e)) => e.print_and_err(),
	}
}

#[get("/register")]
fn get_register(jar: &CookieJar<'_>, origin: &Origin) -> Result<RawHtml<String>, Status> {
	// creating template context
	let mut context = Context::new();

	if let Some(q) = origin.query() {
		for pair in q.segments() {
			match pair {
				("err", "password") => {
					context.insert("error", "Please enter a valid password");
					break;
				}
				("err", "handle") => {
					context.insert("error", "A with this handle already exists");
					break;
				}
				(_, _) => (),
			};
		}
	}

	let session_data: SessionData = jar.into();

	if session_data.user.is_some() {
		return Err(Status::BadRequest);
	}

	// inserting user data
	context.insert("user", &session_data.user);

	// render the template
	match TERA.render("register.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[post("/register", data = "<register_input>")]
async fn register(
	jar: &CookieJar<'_>,
	register_input: Form<AuthInput<'_>>,
) -> Result<Redirect, Status> {
	let (handle, password) = match (register_input.username, register_input.password) {
		(Some(u), Some(p)) => (u, p),
		_ => return Err(Status::BadRequest),
	};

	use AccountError::*;
	match Account::register(handle, password).await {
		Ok(acc) => {
			jar.set_user(acc.id, &acc.handle);
			Ok(Redirect::to("/"))
		}
		Err(Handle(_)) => Ok(Redirect::to("/register?err=handle")),
		Err(Password(_)) => Ok(Redirect::to("/register?err=password")),
		Err(Sqlx(e)) => e.print_and_err(),
	}
}

#[get("/signout")]
fn signout(jar: &CookieJar<'_>) -> Redirect {
	jar.remove_user();
	Redirect::to("/")
}

#[post("/delete_account")]
async fn delete_account(jar: &CookieJar<'_>) -> Result<Redirect, Status> {
	let session_data: SessionData = jar.into();
	let user = match session_data.user {
		Some(user) => user,
		None => return Ok(Redirect::to("/")),
	};

	let account = match Account::find_by_id(user.id).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::Unauthorized),
		Err(e) => return e.print_and_err(),
	};
	match account.delete().await {
		Ok(_) => Ok(signout(jar)),
		Err(e) => e.print_and_err(),
	}
}
