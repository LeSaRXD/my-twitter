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



// global constants
lazy_static! {

	static ref TERA: Tera = match Tera::new("./templates/**/*.html") {
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
		Self { 
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
pub struct BaseTemplatePost {
	pub id: String,
	pub username: String,
	pub body: String,
	pub time: String,
	pub likes: i32,
	pub deleted: bool,
	pub is_reply: bool,
	pub parent_id: Option<String>,
}

impl BaseTemplatePost {

	pub async fn from(post: &database::Post) -> Result<Self, sqlx::Error> {

		Ok(Self {
			id: post.id.to_string(),
			username: database::get_username_by_uuid(&post.poster_id).await?,
			body: post.body.to_owned(),
			time: timestamps::format_timestamp(post.time),
			likes: post.likes,
			deleted: post.deleted,
			is_reply: post.is_reply,
			parent_id: post.parent_id.map(|id| id.to_string()),
		})
		
	}

}

#[derive(Serialize)]
pub struct ReplyTemplatePost {
	base: BaseTemplatePost,
	reply: Option<BaseTemplatePost>,
}




#[launch]
fn rocket() -> _ {

	rocket::build()
		.mount("/", routes![
			get_feed, get_post, create_post, delete_post, get_user,
			get_login, login, get_register, register, signout
		])
		.mount("/static", FileServer::from("./static"))

}



// posts

async fn db_posts_to_template_posts(db_posts: Vec<database::Post>) -> Result<Vec<ReplyTemplatePost>, sqlx::Error> {
	
	future::join_all(
		db_posts
		.iter()
		.map(|post| async {
			Ok(ReplyTemplatePost {
				base: BaseTemplatePost::from(post).await?,
				reply: match database::get_replies(1, post.id).await?.get(0) {
					Some(p) => Some(BaseTemplatePost::from(p).await?),
					None => None,
				},
			})
		})
	)
	.await
	// catch any errors
	.into_iter()
	.collect::<Result<Vec<ReplyTemplatePost>, sqlx::Error>>()

}



#[get("/")]
async fn get_feed(jar: &CookieJar<'_>) -> Result<RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	let db_posts = match database::get_posts(100).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	}.into_iter()
	.filter(|p| !p.deleted && !p.is_reply).collect();

	let posts = match db_posts_to_template_posts(db_posts).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	};
	
	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("feed.html", &context) {
		Ok(s) => Ok(RawHtml(s)),
		Err(e) => {
			println!("{}", e);
			Err(Status::InternalServerError)
		},
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
		Ok(p) => p,
		Err(_) => return Err(Status::NotFound),
	};
	let template_post = match BaseTemplatePost::from(&post).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	};

	context.insert("current_post", &template_post);

	// inserting replies
	let db_replies = match database::get_replies(100, post_id).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	};
	let replies = match db_posts_to_template_posts(db_replies).await {
		Ok(r) => r,
		Err(_) => return Err(Status::InternalServerError),
	};

	context.insert("replies", &replies);



	// rendering the template
	match post.parent_id {
		None => match TERA.render("post.html", &context) {
			Ok(s) => Ok(RawHtml(s)),
			Err(e) => {
				println!("{}", e);
				Err(Status::InternalServerError)
			},
		},
		Some(parent_id) => {

			// inserting parent
			let parent_post = match database::get_post(&parent_id).await {
				Ok(post) => match BaseTemplatePost::from(&post).await {
					Ok(p) => p,
					Err(_) => return Err(Status::InternalServerError),
				},
				Err(_) => return Err(Status::NotFound),
			};

			context.insert("parent_post", &parent_post);

			match TERA.render("reply_post.html", &context) {
				Ok(s) => Ok(RawHtml(s)),
				Err(_) => Err(Status::InternalServerError),
			}

		},
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

	// inserting username
	context.insert("username", &username);

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	let db_posts = match database::get_posts_by_user(100, &username).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	};
	let posts = match db_posts_to_template_posts(db_posts).await {
		Ok(p) => p,
		Err(_) => return Err(Status::InternalServerError),
	};

	context.insert("posts", &posts);

	// rendering the template
	match TERA.render("user.html", &context) {
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
		Err(_) => Ok(Redirect::to("/register?err=login")),
	}

}

#[get("/signout")]
fn signout(jar: &CookieJar<'_>) -> Redirect {

	jar.remove_private(Cookie::named("username"));
	Redirect::to("/")

}
