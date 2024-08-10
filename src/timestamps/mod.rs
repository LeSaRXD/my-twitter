use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> DateTime<Utc> {
	DateTime::from_timestamp_micros(
		SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_micros() as i64,
	)
	.unwrap()
}

pub fn format_timestamp(dt: NaiveDateTime) -> String {
	let dt: DateTime<Utc> = DateTime::from_naive_utc_and_offset(dt, Utc);
	let formatted_time = dt.time().format("%H:%M:%S").to_string();

	let now = current_timestamp();
	if dt.date_naive() == now.date_naive() {
		format!("{} today", formatted_time)
	} else {
		format!("{} {}", formatted_time, dt.date_naive().format("%d/%m/%y"))
	}
}
