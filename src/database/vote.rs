use super::{
	account::Account,
	post::Post,
	types::{AccountId, PostId},
	POOL,
};

pub struct Vote {
	pub voter_id: AccountId,
	pub post_id: PostId,
}

impl Post {
	pub async fn get_voters(&self, user_id: Option<impl Into<i32>>) -> sqlx::Result<Vec<Account>> {
		sqlx::query_as!(
			Account,
			r#"SELECT *,
			(SELECT COUNT(*) FROM follow WHERE user_id = id) AS "following!",
			(SELECT COUNT(*) FROM follow WHERE followed_id = id) AS "followers!",
			EXISTS(SELECT * FROM follow WHERE user_id = $1 AND followed_id = id) AS "followed_by_user!"
			FROM account
			WHERE id IN (SELECT voter_id FROM vote WHERE post_id = $2)"#,
			user_id.map(Into::into),
			i64::from(self.id),
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

impl Account {
	pub async fn get_voted_posts(
		&self,
		user_id: Option<impl Into<i32>>,
	) -> sqlx::Result<Vec<Post>> {
		sqlx::query_as!(
			Post,
			r#"SELECT p.*,
			a.handle AS author_handle,
			a.username AS author_username,
			(SELECT COUNT(*) FROM vote WHERE post_id = p.id) AS "votes!",
			EXISTS(SELECT * FROM vote WHERE post_id = p.id AND voter_id = $2) as "voted_by_user!"
			FROM post p, account a
			WHERE a.id = p.author_id AND
			p.id IN (SELECT post_id FROM vote WHERE voter_id = $1)
			"#,
			i32::from(self.id),
			user_id.map(Into::into),
		)
		.fetch_all(&*POOL)
		.await
	}
}
