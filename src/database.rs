use futures::TryFutureExt;
use sqlx::{Postgres, Pool, FromRow};
use sqlx::postgres::{PgPoolOptions};
use sqlx::types::{Uuid, chrono::NaiveDateTime};
use async_once::AsyncOnce;

use crate::passwords;


const MAX_ITERATIONS: u8 = 100;

lazy_static! {

	static ref CONNECTION_URL: String = dotenv::var("DATABASE_URL").unwrap();
	
	pub static ref POOL: AsyncOnce<Pool<Postgres>> = AsyncOnce::new(
		PgPoolOptions::new()
			.max_connections(5)
			.connect(&CONNECTION_URL).unwrap_or_else(|_| panic!("Error"))
	);

}

#[derive(FromRow)]
pub struct Post {
	pub id: Uuid,
	pub poster_id: Uuid,
	pub body: String,
	pub time: NaiveDateTime,
	pub likes: i32,
	pub deleted: bool,
	pub reply: bool,
	parent_id: Option<Uuid>,
}

pub struct Account {
	pub id: Uuid,
	pub username: String,
	pub password_hash: String,
	pub create_time: NaiveDateTime,
}



// accounts
async fn get_uuid_by_username(username: &String) -> Result<Uuid, sqlx::Error> {

	sqlx::query!("SELECT id FROM account WHERE username=$1", username)
		.fetch_one(POOL.get().await)
		.await.map(|r| r.id)

}

pub async fn get_username_by_uuid(uuid: &Uuid) -> Result<String, sqlx::Error> {

	match sqlx::query!("SELECT username FROM account WHERE id=$1", uuid)
		.fetch_one(POOL.get().await)
		.await {
			Ok(r) => Ok(r.username),
			Err(e) => Err(e)
		}

}

pub async fn login(username: &String, password: &String) -> Result<Option<Account>, sqlx::Error> {

	match sqlx::query_as!(
		Account,
		"SELECT * FROM account WHERE username=$1;",
		username
	)
		.fetch_one(POOL.get().await)
		.await {
			Ok(a) => {
				if passwords::validate_password(password, &a.password_hash, MAX_ITERATIONS) {
					Ok(Some(a))
				} else {
					Ok(None)
				}
			},
			Err(e) => Err(e)
		}

}

pub async fn register(username: &String, password: &String) -> Result<Account, sqlx::Error> {

	match sqlx::query_as!(
		Account,
		"INSERT INTO account (username, password_hash) VALUES ($1, $2) RETURNING id, username, password_hash, create_time;",
		username,
		passwords::encode_password(password, MAX_ITERATIONS)
	)
		.fetch_one(POOL.get().await)
		.await {
			Ok(a) => Ok(a),
			Err(e) => Err(e),
		}

}



// posts
pub async fn get_posts(post_amount: i64) -> Result<Vec<Post>, sqlx::Error> {

	sqlx::query_as!(
		Post, 
		"SELECT * FROM post ORDER BY time DESC LIMIT $1",
		post_amount
	)
	.fetch_all(POOL.get().await)
	.await	

}

pub async fn get_posts_by_user(post_amount: i64, username: &String) -> Result<Vec<Post>, sqlx::Error> {

	sqlx::query_as!(
		Post, 
		"SELECT * FROM post WHERE poster_id=$1 ORDER BY time DESC LIMIT $2",
		get_uuid_by_username(username).await?,
		post_amount
	)
	.fetch_all(POOL.get().await)
	.await	

}

pub async fn get_replies(reply_amount: i64, parent_id: Uuid) -> Result<Vec<Post>, sqlx::Error> {

	sqlx::query_as!(
		Post, 
		"SELECT * FROM post WHERE parent_id=$1 ORDER BY time DESC LIMIT $2",
		parent_id,
		reply_amount
	)
	.fetch_all(POOL.get().await)
	.await	

}




pub async fn get_post(post_id: &Uuid) -> Result<Post, sqlx::Error> {

	sqlx::query_as!(
		Post,
		"SELECT * FROM post WHERE id=$1;",
		post_id
	)
	.fetch_one(POOL.get().await)
	.await

}

pub async fn create_post(username: &String, body: &String, parent_id: Option<Uuid>) -> Result<Uuid, sqlx::Error> {

	let uuid = get_uuid_by_username(username).await?;

	match parent_id {
		Some(parent_id) => Ok(
			sqlx::query!(
				"INSERT INTO post (poster_id, body, reply, parent_id) VALUES($1, $2, TRUE, $3) RETURNING id;",
				uuid,
				body,
				parent_id
			)
			.fetch_one(POOL.get().await)
			.await?
			.id
		),
		None => Ok(
			sqlx::query!(
				"INSERT INTO post (poster_id, body) VALUES($1, $2) RETURNING id;",
				uuid,
				body
			)
			.fetch_one(POOL.get().await)
			.await?
			.id
		),
	}

}

pub async fn delete_post(post_id: &Uuid, username: &String) -> Result<Option<Uuid>, sqlx::Error> {

	let uuid = get_uuid_by_username(username).await?;
	
	match sqlx::query!("UPDATE POST SET deleted=TRUE WHERE id=$1 AND poster_id=$2 RETURNING parent_id;", post_id, uuid)
		.fetch_one(POOL.get().await)
		.await {
			Ok(r) => Ok(r.parent_id),
			Err(e) => Err(e),
		}

}
