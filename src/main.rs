#[macro_use] extern crate lazy_static;
#[macro_use] extern crate rocket;

mod database;
mod timestamps;
mod passwords;

use rocket::{
	response::{content, status, Redirect},
	fs::FileServer,
	http::{CookieJar, Status, Cookie},
	serde::uuid::Uuid,
	form::{Form, FromForm},
	Either::{self, Left, Right}
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
			username: match jar.get_private("username") {
				Some(c) => Some(c.value().to_string()),
				None => None,
			}
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
	body: Option<String>
}



#[derive(Serialize)]
pub struct TemplatePost {
	pub id: String,
	pub username: String,
	pub body: String,
	pub time: String,
	pub likes: i32,
	pub deleted: bool,
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
		})
	}

}



#[launch]
fn rocket() -> _ {

	rocket::build()
		.mount("/", routes![
			get_feed, get_post, get_create_post, create_post, delete_post, get_user,
			get_login, login, get_register, register, signout
		])
		.mount("/static", FileServer::from("../static"))

}



// posts
#[get("/")]
async fn get_feed(jar: &CookieJar<'_>) -> Result<content::RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	// get responses
	let responses = future::join_all(
		match database::get_posts(0, None)
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
		Ok(s) => Ok(content::RawHtml(s)),
		Err(e) => {
			println!("{}", e);
			Err(Status::InternalServerError)
		}
	}

}

#[get("/post/<post_id>")]
async fn get_post(post_id: Uuid) -> Result<String, status::NotFound<String>> {

	let post = match database::get_post(&post_id).await {
		Ok(p) => p,
		Err(_) => return Err(status::NotFound(
			format!("Post with id {} not found", post_id.to_string())
		)),
	};
	// TODO: replace with template
	Ok(format!("Post id: {}\n{}", post_id.to_string(), post.body))

}

#[get("/create_post")]
async fn get_create_post(jar: &CookieJar<'_>) -> Result<content::RawHtml<String>, Either<Redirect, Status>> {

	let userdata: Userdata = jar.into();

	if userdata.username == None {
		return Err(Left(Redirect::to("/login")));
	}

	let mut context = Context::new();
	context.insert("userdata", &userdata);

	match TERA.render("create_post.html", &context) {
		Ok(s) => Ok(content::RawHtml(s)),
		Err(_) => Err(Right(Status::InternalServerError))
	}

}

#[post("/create_post", data = "<post_input>")]
async fn create_post(jar: &CookieJar<'_>, post_input: Form<PostInput>) -> Result<Redirect, Either<Redirect, Status>> {

	let userdata: Userdata = jar.into();
	
	match userdata.username {
		Some(username) => {
			match database::create_post(&username,
				match &post_input.body {
					Some(b) => b,
					None => return Err(Left(Redirect::to("/"))),
				}
			).await {
				Ok(_) => Ok(Redirect::to("/")),
				Err(_) => Err(Right(Status::InternalServerError)),
			}
		},
		None => Err(Left(Redirect::to("/login"))),
	}

}

#[get("/delete_post/<post_id>")]
async fn delete_post(jar: &CookieJar<'_>, post_id: Uuid) -> Result<Redirect, status::BadRequest<String>> {

	let userdata: Userdata = jar.into();
	match userdata.username {
		Some(username) => {
			match database::delete_post(&post_id, &username).await {
				Ok(_) => Ok(Redirect::to("/")),
				Err(_) => Err(status::BadRequest(Some("Couldn't delete post".to_string())))
			}
		},
		None => Err(status::BadRequest(Some("Not logged in".to_string())))
	}

}

#[get("/user/<username>")]
async fn get_user(jar: &CookieJar<'_>, username: String) -> Result<content::RawHtml<String>, Status> {

	let userdata: Userdata = jar.into();

	// creating template context
	let mut context = Context::new();

	// inserting user data
	context.insert("userdata", &userdata);
	
	// inserting posts
	// get responses
	let responses = future::join_all(
		match database::get_posts(0, Some(username))
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
		Ok(s) => Ok(content::RawHtml(s)),
		Err(e) => {
			println!("{}", e);
			Err(Status::InternalServerError)
		}
	}
}



// accounts
#[get("/login")]
fn get_login(jar: &CookieJar<'_>) -> Result<content::RawHtml<String>, Either<Redirect, Status>> {

	let userdata: Userdata = jar.into();

	match userdata.username {
		Some(_) => Err(Left(Redirect::to("/"))),
		None => {
			// creating template context
			let mut context = Context::new();
		
			// inserting user data
			context.insert("userdata", &userdata);

			match TERA.render("login.html", &context) {
				Ok(s) => Ok(content::RawHtml(s)),
				Err(_) => Err(Right(Status::InternalServerError))
			}
			
		}
	}

}

#[post("/login", data = "<login_input>")]
async fn login(jar: &CookieJar<'_>, login_input: Form<AuthInput>) -> Result<Redirect, Either<content::RawHtml<String>, Status>> {

	let userdata = Userdata::from(jar);

	fn render_login(userdata: &Userdata, flash: &str) -> Either<content::RawHtml<String>, Status> {

		let mut context = Context::new();

		context.insert("userdata", &userdata);
		context.insert("flash", &flash);

		match TERA.render("login.html", &context) {
			Ok(s) => return Left(content::RawHtml(s)),
			Err(_) => return Right(Status::InternalServerError)
		}

	}

	match database::login(
		match &login_input.username {
			Some(u) => u,
			None => return Err(render_login(&userdata, "Please enter your login")),
		},
		match &login_input.password {
			Some(p) => p,
			None => return Err(render_login(&userdata, "Please enter your password")),
		}
	).await {
		Ok(a) => {
			match a {
				Some(acc) => {
					jar.add_private(Cookie::new("username", acc.username));
					Ok(Redirect::to("/"))
				},
				None => return Err(render_login(&userdata, "Incorrect password")),
			}
		},
		Err(_) => Err(render_login(&userdata, "Incorrect login"))
	}

}

#[get("/register")]
async fn get_register(jar: &CookieJar<'_>) -> Result<content::RawHtml<String>, Either<Redirect, Status>> {

	let userdata: Userdata = jar.into();

	match userdata.username {
		Some(_) => Err(Left(Redirect::to("/"))),
		None => {
			// creating template context
			let mut context = Context::new();
		
			// inserting user data
			context.insert("userdata", &userdata);

			match TERA.render("register.html", &context) {
				Ok(s) => Ok(content::RawHtml(s)),
				Err(_) => Err(Right(Status::InternalServerError))
			}
			
		}
	}

}

#[post("/register", data = "<register_input>")]
async fn register(jar: &CookieJar<'_>, register_input: Form<AuthInput>) -> Result<Redirect, Either<content::RawHtml<String>, Status>> {

	let userdata = Userdata::from(jar);

	fn render_register(userdata: &Userdata, flash: &str) -> Either<content::RawHtml<String>, Status> {

		let mut context = Context::new();

		context.insert("userdata", &userdata);
		context.insert("flash", &flash);

		match TERA.render("register.html", &context) {
			Ok(s) => return Left(content::RawHtml(s)),
			Err(_) => return Right(Status::InternalServerError)
		}

	}

	match database::register(
		match &register_input.username {
			Some(u) => u,
			None => return Err(render_register(&userdata, "Please enter your login")),
		},
		match &register_input.password {
			Some(p) => p,
			None => return Err(render_register(&userdata, "Please enter your password")),
		}
	).await {
		Ok(acc) =>  {
			jar.add_private(Cookie::new("username", acc.username));
			Ok(Redirect::to("/"))
		},
		Err(_) => Err(render_register(&userdata, "User with that login already exists"))
	}

}

#[get("/signout")]
fn signout(jar: &CookieJar<'_>) -> Redirect {

	jar.remove_private(Cookie::named("username"));
	Redirect::to("/")

}
