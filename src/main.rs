#[macro_use] extern crate lazy_static;
#[macro_use] extern crate rocket;

mod database;
mod timestamps;
mod passwords;

use rocket::{
	response::{content::RawHtml, status, Redirect},
	fs::FileServer,
	http::{CookieJar, Status, Cookie},
	serde::uuid::Uuid,
	form::{Form, FromForm},
};
use tera::{Tera, Context};
use serde::Serialize;
use futures::future;



lazy_static! {

	static ref TERA: Tera = match Tera::new("../templates/**/*.html") {
		Ok(t) => t,
		Err(e) => panic!("Error!\n{}", e),
	};

}



#[derive(Serialize)]
struct Userdata {
	username: Option<String>,
}
impl From<&CookieJar<'_>> for Userdata {
	fn from(jar: &CookieJar) -> Self {
		Userdata { 
			username: jar.get_private("username").map(|c| c.value().to_string())
		}
	}
}

#[derive(FromForm)]
struct AuthInput {
	username: Option<String>,
	password: Option<String>
}

#[derive(FromForm)]
struct PostInput {
	body: Option<String>,
	parent_id: Option<Uuid>,
}



#[derive(Serialize)]
pub struct TemplatePost {
	pub id: String,
	pub username: String,
	pub body: String,
	pub time: String,
	pub likes: i32,
	pub deleted: bool,
	pub reply: bool,
}
impl TemplatePost {

	pub async fn from(post: &database::Post) -> Result<Self, sqlx::Error> {
		Ok(TemplatePost {
			id: post.id.to_string(),
			username: database::get_username_by_uuid(&post.poster_id).await?,
			body: post.body.to_owned(),
			time: timestamps::format_timestamp(post.time),
			likes: post.likes,
			deleted: post.deleted,
			reply: post.reply,
		})
	}

}



#[launch]
fn rocket() -> _ {

	rocket::build()
		.mount("/", routes![
			get_feed, get_post, create_post, delete_post, get_user,
			get_login, login, get_register, register, signout
		])
		.mount("/static", FileServer::from("../static"))

}



// posts
#[get("/")]
async fn get_feed(jar: &CookieJar<'_>) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	let posts = match future::join_all(
		// get posts from database
		match database::get_posts(100).await {
			Ok(p) => p,
			Err(_) => return Err(Status::InternalServerError),
		}
		// convert them into template posts
		.iter()
		.map(|post| async { TemplatePost::from(post).await })
	)
	.await
	// catch any errors
	.into_iter()
	.collect::<Result<Vec<TemplatePost>, sqlx::Error>>() {
		Ok(posts) => posts,
		Err(_) => return Err(Status::InternalServerError),
	};
	
	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("feed.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => {
			println!("{}", e);
			Err(Status::InternalServerError)
		}
	}

}

#[get("/post/<post_id>")]
async fn get_post(jar: &CookieJar<'_>, post_id: Uuid) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);

	// inserting post
	let post = match database::get_post(&post_id).await {
		Ok(post) => match TemplatePost::from(&post).await {
			Ok(tp) => tp,
			Err(_) => return Err(Status::InternalServerError),
		},
		Err(_) => return Err(Status::NotFound),
	};

	context.insert("post", &post);

	// inserting replies
	let replies = match future::join_all(
		// get posts from database
		match database::get_replies(100, post_id).await {
			Ok(p) => p,
			Err(_) => return Err(Status::InternalServerError),
		}
		// convert them into template posts
		.iter()
		.map(|post| async { TemplatePost::from(post).await })
	)
	.await
	// catch any errors
	.into_iter()
	.collect::<Result<Vec<TemplatePost>, sqlx::Error>>() {
		Ok(posts) => posts,
		Err(_) => return Err(Status::InternalServerError),
	};

	context.insert("replies", &replies);

	// rendering the template
	match TERA.render("post.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(_) => Err(Status::InternalServerError),
	}

}

#[post("/create_post", data = "<post_input>")]
async fn create_post(jar: &CookieJar<'_>, post_input: Form<PostInput>) -> Result<Redirect, Status> {

	let userdata: Userdata = jar.into();
	
	match userdata.username {
		Some(username) => {
			match database::create_post(&username,
				match &post_input.body {
					Some(b) => b,
					None => return Err(Status::BadRequest),
				},
				post_input.parent_id,
			).await {
				Ok(_) => Ok(Redirect::to(
					match post_input.parent_id {
						Some(id) => format!("/post/{}", id),
						None => "/".to_string(),
					}
				)),
				Err(_) => Err(Status::InternalServerError),
			}
		},
		None => Err(Status::Unauthorized),
	}

}

#[get("/delete_post/<post_id>")]
async fn delete_post(jar: &CookieJar<'_>, post_id: Uuid) -> Result<Redirect, Status> {

	let userdata: Userdata = jar.into();
	match userdata.username {
		Some(username) => {
			match database::delete_post(&post_id, &username).await {
				Ok(parent_id) => Ok(Redirect::to(
					match parent_id {
						Some(id) => format!("/post/{}", id),
						None => "/".to_string(),
					}
				)),
				Err(_) => Err(Status::BadRequest)
			}
		},
		None => Err(Status::Unauthorized)
	}

}

#[get("/user/<username>")]
async fn get_user(jar: &CookieJar<'_>, username: String) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	// get responses
	let responses = future::join_all(
		match database::get_posts_by_user(100, &username)
		.await {
			Ok(p) => p,
			Err(_) => { return Err(Status::InternalServerError); },
		}
		.iter()
		.map(|post| async { TemplatePost::from(post).await } )
	).await;
	// look throigh each response in case there's an error
	let mut posts: Vec<TemplatePost> = Vec::new();
	for res in responses {
		match res {
			Ok(p) => posts.push(p),
			Err(_) => return Err(Status::InternalServerError),
		}
	}
	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("feed.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(_) => Err(Status::InternalServerError)
	}
}



// accounts
#[get("/login")]
fn get_login(jar: &CookieJar<'_>) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	match userdata.username {
		Some(_) => Err(Status::BadRequest),
		None => {
			// creating template context
			let mut context = Context::new();
		
			// inserting user data
			context.insert("userdata", &userdata);

			match TERA.render("login.html", &context) {
				Ok(s) => Ok(RawHtml(s)),
				Err(_) => Err(Status::InternalServerError)
			}
			
		}
	}

}

#[post("/login", data = "<login_input>")]
async fn login(jar: &CookieJar<'_>, login_input: Form<AuthInput>) -> Result<Redirect, status::BadRequest<&'static str>> {

	match database::login(
		match &login_input.username {
			Some(u) => u,
			None => return Err(status::BadRequest(Some("No login provided"))),
		},
		match &login_input.password {
			Some(p) => p,
			None => return Err(status::BadRequest(Some("No password provided"))),
		}
	).await {
		Ok(a) => {
			match a {
				Some(acc) => {
					jar.add_private(Cookie::new("username", acc.username));
					Ok(Redirect::to("/"))
				},
				None => Ok(Redirect::to("/login?err=pw")),
			}
		},
		Err(_) => Ok(Redirect::to("/login?err=login"))
	}

}

#[get("/register")]
fn get_register(jar: &CookieJar<'_>) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	match userdata.username {
		Some(_) => Err(Status::BadRequest),
		None => {
			// creating template context
			let mut context = Context::new();
		
			// inserting user data
			context.insert("userdata", &userdata);

			match TERA.render("register.html", &context) {
				Ok(s) => Ok(RawHtml(s)),
				Err(_) => Err(Status::InternalServerError)
			}
			
		}
	}

}

#[post("/register", data = "<register_input>")]
async fn register(jar: &CookieJar<'_>, register_input: Form<AuthInput>) -> Result<Redirect, status::BadRequest<&'static str>> {

	match database::register(
		match &register_input.username {
			Some(u) => u,
			None => return Err(status::BadRequest(Some("No login provided"))),
		},
		match &register_input.password {
			Some(p) => p,
			None => return Err(status::BadRequest(Some("No password provided"))),
		}
	).await {
		Ok(acc) => {
			jar.add_private(Cookie::new("username", acc.username));
			Ok(Redirect::to("/"))
		},
		Err(_) => Ok(Redirect::to("/login?err=login")),
	}

}

#[get("/signout")]
fn signout(jar: &CookieJar<'_>) -> Redirect {

	jar.remove_private(Cookie::named("username"));
	Redirect::to("/")

}
