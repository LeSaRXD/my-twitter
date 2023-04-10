use sqlx::types::chrono::NaiveDateTime;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> NaiveDateTime {
	NaiveDateTime::from_timestamp_micros(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as i64).unwrap()
}

pub fn format_timestamp(dt: NaiveDateTime) -> String {
	
	let formatted_time = dt.time().format("%H:%M:%S").to_string();

	let now = current_timestamp();	
	if dt.date() == now.date() {
		format!("{} today", formatted_time)
	} else {
		format!("{} {}", formatted_time, dt.date().format("%d/%m/%y"))
	}

}