pub mod account;
pub mod post;
pub mod types;
pub mod vote;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, PgPool};
use std::str::FromStr;
use std::sync::LazyLock;

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
