#[macro_use]
extern crate rocket;

mod crypto;
mod database;
mod helpers;
mod timestamps;

use database::{
	account::{Account, AccountError},
	post::Post,
	types::{AccountId, PostId},
};
use futures::future;
use helpers::{CookieJarHelper, ErrorHelper};
use rocket::{
	form::{Form, FromForm},
	fs::FileServer,
	http::{uri::Origin, Cookie, CookieJar, Status},
	response::{content::RawHtml, Redirect},
};
use rocket_dyn_templates::tera::{Context, ErrorKind, Tera};
use serde::Serialize;
use std::sync::LazyLock;

// global constants
static TERA: LazyLock<Tera> = LazyLock::new(|| match Tera::new("./templates/**/*.html") {
	Ok(t) => t,
	Err(e) => {
		panic!(
			"Could not load templates from ./templates\n{}",
			if let ErrorKind::Msg(m) = e.kind {
				m.to_string()
			} else {
				e.to_string()
			}
		);
	}
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
impl From<&SessionUser> for i32 {
	fn from(value: &SessionUser) -> Self {
		value.id.into()
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
impl From<Post> for BaseTemplatePost {
	fn from(value: Post) -> Self {
		Self {
			id: value.id.0,
			author_id: value.author_id.0,
			author_name: value
				.author_username
				.map(String::into_boxed_str)
				.unwrap_or_else(|| value.author_handle.clone()),
			author_handle: value.author_handle,
			body: value.body,
			create_time: timestamps::format_timestamp(value.create_time).into_boxed_str(),
			likes: value.votes.0,
			liked_by_user: value.voted_by_user,
			parent_id: value.parent_id.0.map(Into::into),
		}
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
			let mut replies = post.get_replies(1, user.as_ref()).await?;
			let reply = replies.pop().map(Into::into);
			Ok(Self {
				base: post.into(),
				reply,
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
	let _ = &*TERA;

	rocket::build()
		.mount(
			"/",
			routes![
				favicon,
				get_feed,
				get_post,
				get_post_likes,
				create_post,
				delete_post,
				like_post,
				get_user,
				get_user_likes,
				follow_user,
				unfollow_user,
				get_login,
				get_register,
				login,
				register,
				signout,
				delete_account,
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
	let SessionData { user } = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &user);

	// inserting posts
	let posts = match Post::get_recent(100, user.as_ref()).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	let posts = match ReplyTemplatePost::from_posts(posts, &user).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("post/feed.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[get("/post/<post_id>")]
async fn get_post(jar: &CookieJar<'_>, post_id: PostId) -> Result<RawHtml<String>, Status> {
	let SessionData { user }: SessionData = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &user);

	// inserting post
	let post = match Post::find_by_id(post_id, user.as_ref()).await {
		Ok(Some(p)) => p,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};
	// inserting replies
	let replies = match post.get_replies(100, user.as_ref()).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	let replies = match ReplyTemplatePost::from_posts(replies, &user).await {
		Ok(r) => r,
		Err(e) => return e.print_and_err(),
	};
	context.insert("replies", &replies);

	let post: BaseTemplatePost = post.into();
	context.insert("base_post", &post);

	// rendering the template
	match post.parent_id {
		None => match TERA.render("post/index.html", &context) {
			Ok(s) => Ok(RawHtml(s)),
			Err(e) => e.print_and_err(),
		},
		Some(parent_id) => {
			// inserting parent
			let parent_post = match Post::find_by_id(PostId(parent_id), user.as_ref()).await {
				Ok(Some(post)) => BaseTemplatePost::from(post),
				Ok(None) => return Err(Status::NotFound),
				Err(e) => return e.print_and_err(),
			};

			context.insert("parent_post", &parent_post);

			match TERA.render("post/reply.html", &context) {
				Ok(s) => Ok(RawHtml(s)),
				Err(e) => e.print_and_err(),
			}
		}
	}
}

#[get("/post/<post_id>/likes")]
async fn get_post_likes(jar: &CookieJar<'_>, post_id: PostId) -> Result<RawHtml<String>, Status> {
	let SessionData { user } = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &user);

	// inserting post
	let post = match Post::find_by_id(post_id, user.as_ref()).await {
		Ok(Some(p)) => p,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};
	// inserting voters
	let likes = match post.get_voters(user.as_ref()).await {
		Ok(v) => v,
		Err(e) => return e.print_and_err(),
	};
	context.insert("likes", &likes);

	// inserting base post
	let base_post: BaseTemplatePost = post.into();
	context.insert("base_post", &base_post);

	match TERA.render("post/likes.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[post("/create_post", data = "<post_input>")]
async fn create_post(jar: &CookieJar<'_>, post_input: Form<PostInput>) -> Result<Redirect, Status> {
	let SessionData { user } = jar.into();

	let body = match &post_input.body {
		Some(b) => b.as_ref(),
		None => return Err(Status::BadRequest),
	};

	let user = match user {
		Some(user) => user,
		None => return Err(Status::Unauthorized),
	};
	let account = match Account::find_by_id(user.id, None::<i32>).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::Unauthorized),
		Err(e) => return e.print_and_err(),
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
	let SessionData { user } = jar.into();

	let user = match user {
		Some(user) => user,
		None => return Err(Status::Unauthorized),
	};
	let post = match Post::find_by_id(post_id, Some(&user)).await {
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
	let SessionData { user } = jar.into();

	let user = match user {
		Some(user) => user,
		None => return Status::Unauthorized,
	};
	let account = match Account::find_by_id(user.id, None::<i32>).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Status::Unauthorized,
		Err(e) => return e.print_and_status(),
	};

	let post = match Post::find_by_id(post_id, Some(&user)).await {
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

// users

#[get("/user/<handle>")]
async fn get_user(jar: &CookieJar<'_>, handle: &str) -> Result<RawHtml<String>, Status> {
	let SessionData { user } = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &user);

	let account = match Account::find_by_handle(handle, user.as_ref()).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};

	// inserting handle
	context.insert("account", &account);

	// inserting posts
	let posts = match account.get_posts(100, false, user.as_ref()).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	let posts = match ReplyTemplatePost::from_posts(posts, &user).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("account/index.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

#[get("/user/<handle>/likes")]
async fn get_user_likes(jar: &CookieJar<'_>, handle: &str) -> Result<RawHtml<String>, Status> {
	let SessionData { user } = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("user", &user);

	let account = match Account::find_by_handle(handle, user.as_ref()).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};

	// inserting handle
	context.insert("account", &account);

	// inserting posts
	let posts = match account.get_voted_posts(user.as_ref()).await {
		Ok(p) => p,
		Err(e) => return e.print_and_err(),
	};
	let posts: Vec<BaseTemplatePost> = posts.into_iter().map(Into::into).collect();

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("account/likes.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => e.print_and_err(),
	}
}

async fn follow_or_unfollow(
	jar: &CookieJar<'_>,
	handle: &str,
	follow: bool,
) -> Result<Redirect, Status> {
	let SessionData { user } = jar.into();

	let user = match user {
		Some(user) => user,
		None => return Err(Status::Unauthorized),
	};

	let account = match Account::find_by_id(user.id, Some(&user)).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};

	let to_follow = match Account::find_by_handle(handle, Some(&user)).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::NotFound),
		Err(e) => return e.print_and_err(),
	};

	if account.id == to_follow.id {
		return Err(Status::Forbidden);
	}

	let result = if follow {
		account.follow(to_follow.id).await.map(|_| ())
	} else {
		account.unfollow(to_follow.id).await
	};

	match result {
		Ok(()) => Ok(Redirect::to(format!("/user/{handle}"))),
		Err(e) => e.print_and_err(),
	}
}

#[get("/user/<handle>/follow")]
async fn follow_user(jar: &CookieJar<'_>, handle: &str) -> Result<Redirect, Status> {
	follow_or_unfollow(jar, handle, true).await
}

#[get("/user/<handle>/unfollow")]
async fn unfollow_user(jar: &CookieJar<'_>, handle: &str) -> Result<Redirect, Status> {
	follow_or_unfollow(jar, handle, false).await
}

// accounts

#[get("/login")]
fn get_login(jar: &CookieJar<'_>, origin: &Origin) -> Result<RawHtml<String>, Status> {
	let SessionData { user } = jar.into();

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

	if user.is_some() {
		return Err(Status::BadRequest);
	}

	// inserting user data
	context.insert("user", &user);

	// render the template
	match TERA.render("user/login.html", &context) {
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

	let SessionData { user } = jar.into();

	if user.is_some() {
		return Err(Status::BadRequest);
	}

	// inserting user data
	context.insert("user", &user);

	// render the template
	match TERA.render("user/register.html", &context) {
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
	let SessionData { user } = jar.into();

	let user = match user {
		Some(user) => user,
		None => return Ok(Redirect::to("/")),
	};

	let account = match Account::find_by_id(user.id, None::<i32>).await {
		Ok(Some(acc)) => acc,
		Ok(None) => return Err(Status::Unauthorized),
		Err(e) => return e.print_and_err(),
	};
	match account.delete().await {
		Ok(_) => Ok(signout(jar)),
		Err(e) => e.print_and_err(),
	}
}
