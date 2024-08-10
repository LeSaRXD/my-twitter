pub mod types;

use crate::crypto;
use chrono::NaiveDateTime;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;
use std::sync::LazyLock;
use types::{AccountId, OptPostId, PgU64, PostId};

const MAX_ITERATIONS: u8 = 100;

static POOL: LazyLock<PgPool> = LazyLock::new(|| {
	let conn_url = dotenvy::var("DATABASE_URL").unwrap();
	let options = PgConnectOptions::from_str(&conn_url)
		.unwrap()
		.disable_statement_logging();

	futures::executor::block_on(async move {
		PgPoolOptions::new()
			.connect_with(options)
			.await
			.expect("Could not connect to database")
	})
});

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

// post actions for account
impl Account {
	pub async fn get_posts(&self, limit: u64, include_replies: bool) -> sqlx::Result<Vec<Post>> {
		sqlx::query_as!(
			Post,
			r#"SELECT *,
			(SELECT COUNT(*) FROM vote WHERE post_id = id) AS "votes!"
			FROM post
			WHERE author_id = $1 AND
			(
				parent_id IS NULL OR
				(parent_id IS NOT NULL) = $2
			)
			ORDER BY create_time DESC
			LIMIT $3"#,
			i32::from(self.id),
			include_replies,
			limit as i64,
		)
		.fetch_all(&*POOL)
		.await
	}

	pub async fn create_post(
		&self,
		body: &str,
		parent_id: Option<impl Into<i64>>,
	) -> sqlx::Result<Post> {
		sqlx::query_as!(
			Post,
			r#"INSERT INTO post (author_id, body, parent_id)
			VALUES ($1, $2, $3)
			RETURNING *, 0 as "votes!""#,
			i32::from(self.id),
			body,
			parent_id.map(Into::into),
		)
		.fetch_one(&*POOL)
		.await
	}
	pub async fn post(&self, body: &str) -> sqlx::Result<Post> {
		self.create_post(body, None as Option<i64>).await
	}
	pub async fn reply(&self, body: &str, parent_id: impl Into<i64>) -> sqlx::Result<Post> {
		self.create_post(body, Some(parent_id)).await
	}

	pub async fn add_vote(&self, post_id: impl Into<i64>) -> sqlx::Result<Vote> {
		sqlx::query_as!(
			Vote,
			r#"INSERT INTO vote (voter_id, post_id)
			VALUES ($1, $2)
			ON CONFLICT (voter_id, post_id) DO NOTHING
			RETURNING *"#,
			i32::from(self.id),
			post_id.into(),
		)
		.fetch_one(&*POOL)
		.await
	}
	pub async fn remove_vote(&self, post_id: impl Into<i64>) -> sqlx::Result<()> {
		sqlx::query_as!(
			Vote,
			r#"DELETE FROM vote WHERE voter_id = $1 AND post_id = $2"#,
			i32::from(self.id),
			post_id.into(),
		)
		.execute(&*POOL)
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

pub struct Post {
	pub id: PostId,
	pub author_id: AccountId,
	pub body: Box<str>,
	pub create_time: NaiveDateTime,
	pub parent_id: OptPostId,
	pub votes: PgU64,
}
impl Post {
	pub async fn find_by_id(post_id: impl Into<i64>) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(
			Post,
			r#"SELECT *,
			(SELECT COUNT(*) FROM vote WHERE post_id = id) as "votes!"
			FROM post WHERE id = $1"#,
			post_id.into(),
		)
		.fetch_optional(&*POOL)
		.await
	}
	pub async fn get_recent(limit: u64) -> sqlx::Result<Vec<Self>> {
		sqlx::query_as!(
			Post,
			r#"SELECT *,
			(SELECT COUNT(*) FROM vote WHERE post_id = id) AS "votes!"
			FROM post
			ORDER BY create_time DESC
			LIMIT $1"#,
			limit as i64,
		)
		.fetch_all(&*POOL)
		.await
	}

	pub async fn delete(self) -> sqlx::Result<Option<PostId>> {
		sqlx::query_scalar!(
			r#"DELETE FROM post WHERE id = $1 RETURNING parent_id"#,
			i64::from(self.id),
		)
		.fetch_one(&*POOL)
		.await
		.map(|o| o.map(Into::into))
	}

	pub async fn get_votes(&self) -> sqlx::Result<Vec<Vote>> {
		sqlx::query_as!(
			Vote,
			r#"SELECT * FROM vote WHERE post_id = $1"#,
			i64::from(self.id),
		)
		.fetch_all(&*POOL)
		.await
	}
	pub async fn get_replies(&self, limit: u64) -> sqlx::Result<Vec<Post>> {
		sqlx::query_as!(
			Post,
			r#"SELECT *,
			(SELECT COUNT(*) FROM vote WHERE post_id = id) AS "votes!"
			FROM post WHERE parent_id = $1
			ORDER BY "votes!" DESC
			LIMIT $2"#,
			i64::from(self.id),
			limit as i64,
		)
		.fetch_all(&*POOL)
		.await
	}

	pub async fn voted_by(&self, account_id: impl Into<i32>) -> sqlx::Result<bool> {
		sqlx::query_scalar!(
			r#"SELECT EXISTS(
				SELECT * FROM vote WHERE post_id = $1 AND voter_id = $2
			) AS "exists!""#,
			i64::from(self.id),
			account_id.into(),
		)
		.fetch_one(&*POOL)
		.await
	}
}

pub struct Vote {
	#[allow(dead_code)]
	voter_id: AccountId,
	#[allow(dead_code)]
	post_id: PostId,
}
