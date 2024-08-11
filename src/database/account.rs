use super::{types::AccountId, MAX_ITERATIONS, POOL};
use crate::crypto;
use chrono::NaiveDateTime;
use serde::Serialize;

pub enum AccountError<'a> {
	Handle(&'a str),
	Password(&'a str),
	Sqlx(sqlx::Error),
}
impl<'a> From<sqlx::Error> for AccountError<'a> {
	fn from(value: sqlx::Error) -> Self {
		Self::Sqlx(value)
	}
}

#[derive(Serialize)]
pub struct Account {
	pub id: AccountId,
	pub handle: String,
	pub username: Option<String>,
	pub password_hash: Box<[u8]>,
	pub create_time: NaiveDateTime,
}
// account actions
impl Account {
	pub async fn find_by_id(id: impl Into<i32>) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(Account, r#"SELECT * FROM account WHERE id = $1"#, id.into(),)
			.fetch_optional(&*POOL)
			.await
	}
	pub async fn find_by_handle(handle: &str) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(
			Account,
			r#"SELECT * FROM account WHERE handle = $1"#,
			handle,
		)
		.fetch_optional(&*POOL)
		.await
	}

	pub async fn login<'a>(handle: &'a str, password: &'a str) -> Result<Self, AccountError<'a>> {
		use AccountError::*;

		match sqlx::query_as!(
			Account,
			r#"SELECT * FROM account WHERE handle = $1"#,
			handle,
		)
		.fetch_optional(&*POOL)
		.await?
		{
			Some(acc) => {
				if crypto::validate_password(password, &acc.password_hash, MAX_ITERATIONS) {
					Ok(acc)
				} else {
					Err(Password(password))
				}
			}
			None => Err(Handle(handle)),
		}
	}
	pub async fn register<'a>(
		handle: &'a str,
		password: &'a str,
	) -> Result<Self, AccountError<'a>> {
		use sqlx::Error::Database;
		use AccountError::*;

		// TODO: password check

		sqlx::query_as!(
			Account,
			r#"INSERT INTO account (handle, password_hash)
			VALUES ($1, $2)
			RETURNING *"#,
			handle,
			&crypto::encode_password(password, MAX_ITERATIONS)
		)
		.fetch_one(&*POOL)
		.await
		.map_err(|e| match e {
			Database(err) => {
				if err.is_unique_violation() {
					Handle(handle)
				} else {
					Sqlx(Database(err))
				}
			}
			e => Sqlx(e),
		})
	}

	pub async fn delete(self) -> sqlx::Result<()> {
		sqlx::query!(r#"DELETE FROM account WHERE id = $1"#, i32::from(self.id),)
			.fetch_one(&*POOL)
			.await
			.map(|_| ())
	}
}

// helper methods
impl Account {
	pub fn display_name(&self) -> &str {
		self.username.as_ref().unwrap_or(&self.handle)
	}
}
