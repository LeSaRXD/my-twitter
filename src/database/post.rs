use chrono::NaiveDateTime;

use super::{
	account::Account,
	types::{AccountId, OptPostId, PgU64, PostId},
	vote::Vote,
	POOL,
};

#[derive(Debug)]
pub struct Post {
	pub id: PostId,
	pub author_id: AccountId,
	pub author_handle: Box<str>,
	pub author_username: Option<String>,
	pub body: Box<str>,
	pub create_time: NaiveDateTime,
	pub parent_id: OptPostId,
	pub votes: PgU64,
	pub voted_by_user: bool,
}
impl Post {
	pub async fn find_by_id(
		post_id: impl Into<i64>,
		user_id: Option<impl Into<i32>>,
	) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(
			Post,
			r#"SELECT p.*,
			a.handle as author_handle,
			a.username as author_username,
			(SELECT COUNT(*) FROM vote WHERE post_id = p.id) as "votes!",
			EXISTS(SELECT * FROM vote WHERE post_id = p.id AND voter_id = $2) as "voted_by_user!"
			FROM post p, account a
			WHERE p.id = $1 AND a.id = p.author_id"#,
			post_id.into(),
			user_id.map(Into::into),
		)
		.fetch_optional(&*POOL)
		.await
	}
	pub async fn get_recent(
		limit: u64,
		user_id: Option<impl Into<i32>>,
	) -> sqlx::Result<Vec<Self>> {
		sqlx::query_as!(
			Post,
			r#"SELECT p.*,
			a.handle as author_handle,
			a.username as author_username,
			(SELECT COUNT(*) FROM vote WHERE post_id = p.id) as "votes!",
			EXISTS(SELECT * FROM vote WHERE post_id = p.id AND voter_id = $2) as "voted_by_user!"
			FROM post p, account a
			WHERE a.id = p.author_id AND
			p.parent_id IS NULL
			ORDER BY create_time DESC
			LIMIT $1"#,
			limit as i64,
			user_id.map(Into::into),
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

	pub async fn get_replies(
		&self,
		limit: u64,
		user_id: Option<impl Into<i32>>,
	) -> sqlx::Result<Vec<Post>> {
		sqlx::query_as!(
			Post,
			r#"SELECT p.*,
			a.handle as author_handle,
			a.username as author_username,
			(SELECT COUNT(*) FROM vote WHERE post_id = p.id) as "votes!",
			EXISTS(SELECT * FROM vote WHERE post_id = p.id AND voter_id = $3) as "voted_by_user!"
			FROM post p, account a
			WHERE p.parent_id = $1 AND a.id = p.author_id
			ORDER BY "votes!" DESC
			LIMIT $2"#,
			i64::from(self.id),
			limit as i64,
			user_id.map(Into::into),
		)
		.fetch_all(&*POOL)
		.await
	}
}

// post actions for account
impl Account {
	pub async fn get_posts(
		&self,
		limit: u64,
		include_replies: bool,
		user_id: Option<impl Into<i32>>,
	) -> sqlx::Result<Vec<Post>> {
		sqlx::query_as!(
			Post,
			r#"SELECT p.*,
			a.handle AS author_handle,
			a.username AS author_username,
			(SELECT COUNT(*) FROM vote WHERE post_id = p.id) AS "votes!",
			EXISTS(SELECT * FROM vote WHERE post_id = p.id AND voter_id = $4) as "voted_by_user!"
			FROM post p, account a
			WHERE a.id = p.author_id AND
			p.author_id = $1 AND
			(
				p.parent_id IS NULL OR
				(p.parent_id IS NOT NULL) = $2
			)
			ORDER BY create_time DESC
			LIMIT $3"#,
			i32::from(self.id),
			include_replies,
			limit as i64,
			user_id.map(Into::into),
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
			r#"WITH inserted AS (
				INSERT INTO post (author_id, body, parent_id)
				VALUES ($1, $2, $3)
				RETURNING *
			)
			SELECT p.*,
			a.handle AS author_handle,
			a.username AS author_username,
			0 AS "votes!",
			FALSE AS "voted_by_user!"
			FROM inserted p, account a
			WHERE a.id = p.author_id"#,
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
