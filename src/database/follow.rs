use super::{account::Account, types::AccountId, POOL};

pub struct Follow {
	pub user_id: AccountId,
	pub followed_id: AccountId,
}

impl Account {
	pub async fn follow(&self, follow_id: impl Into<i32>) -> sqlx::Result<Follow> {
		sqlx::query_as!(
			Follow,
			r#"INSERT INTO follow (user_id, followed_id)
			VALUES ($1, $2)
			ON CONFLICT (user_id, followed_id) DO NOTHING
			RETURNING *"#,
			i32::from(self.id),
			follow_id.into(),
		)
		.fetch_one(&*POOL)
		.await
	}
	pub async fn unfollow(&self, followed_id: impl Into<i32>) -> sqlx::Result<()> {
		sqlx::query!(
			r#"DELETE FROM follow
			WHERE user_id = $1 AND followed_id = $2"#,
			i32::from(self.id),
			followed_id.into(),
		)
		.execute(&*POOL)
		.await
		.map(|_| ())
	}
}
