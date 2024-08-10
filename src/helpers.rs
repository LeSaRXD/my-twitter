use std::fmt::Display;

use rocket::http::{Cookie, CookieJar, Status};

use crate::database::types::AccountId;

pub trait ErrorHelper {
	fn print_and_err<T>(self) -> Result<T, Status>
	where
		Self: Sized + Display,
	{
		Err(self.print_and_status())
	}
	fn print_and_status(self) -> Status
	where
		Self: Sized + Display,
	{
		eprintln!("{self}");
		Status::InternalServerError
	}
}
impl ErrorHelper for sqlx::Error {}
impl ErrorHelper for rocket_dyn_templates::tera::Error {}

pub trait CookieJarHelper {
	fn set_user(&self, id: AccountId, handle: &str);
	fn remove_user(&self);
}
impl<'a> CookieJarHelper for CookieJar<'a> {
	fn set_user(&self, id: AccountId, handle: &str) {
		self.add_private(Cookie::new("id", id.0.to_string()));
		self.add_private(Cookie::new("handle", handle.to_owned()));
	}
	fn remove_user(&self) {
		self.remove_private(Cookie::from("id"));
		self.remove_private(Cookie::from("handle"));
	}
}
