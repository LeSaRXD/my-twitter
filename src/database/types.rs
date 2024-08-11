use std::num::ParseIntError;

use super::{account::Account, post::Post};
use rocket::request::FromParam;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PgU64(pub u64);
impl From<u64> for PgU64 {
	fn from(value: u64) -> Self {
		Self(value)
	}
}
impl From<i64> for PgU64 {
	fn from(value: i64) -> Self {
		Self(value as u64)
	}
}
impl From<i32> for PgU64 {
	fn from(value: i32) -> Self {
		Self(value as u64)
	}
}
impl From<PgU64> for u64 {
	fn from(value: PgU64) -> Self {
		value.0
	}
}
impl From<PgU64> for i64 {
	fn from(value: PgU64) -> Self {
		value.0 as i64
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct AccountId(pub u32);
impl From<u32> for AccountId {
	fn from(value: u32) -> Self {
		Self(value)
	}
}
impl From<i32> for AccountId {
	fn from(value: i32) -> Self {
		Self(value as u32)
	}
}
impl From<Account> for AccountId {
	fn from(value: Account) -> Self {
		value.id
	}
}
impl From<&Account> for AccountId {
	fn from(value: &Account) -> Self {
		value.id
	}
}
impl From<AccountId> for i32 {
	fn from(value: AccountId) -> Self {
		value.0 as i32
	}
}
impl<'a> FromParam<'a> for AccountId {
	type Error = ParseIntError;
	fn from_param(param: &'a str) -> Result<Self, Self::Error> {
		Ok(Self(param.parse()?))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromForm)]
pub struct PostId(pub u64);
impl From<u64> for PostId {
	fn from(value: u64) -> Self {
		Self(value)
	}
}
impl From<i64> for PostId {
	fn from(value: i64) -> Self {
		Self(value as u64)
	}
}
impl From<Post> for PostId {
	fn from(value: Post) -> Self {
		value.id
	}
}
impl From<&Post> for PostId {
	fn from(value: &Post) -> Self {
		value.id
	}
}
impl From<PostId> for u64 {
	fn from(value: PostId) -> Self {
		value.0
	}
}
impl From<PostId> for i64 {
	fn from(value: PostId) -> Self {
		value.0 as i64
	}
}

impl<'a> FromParam<'a> for PostId {
	type Error = ParseIntError;
	fn from_param(param: &'a str) -> Result<Self, Self::Error> {
		Ok(Self(param.parse()?))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OptPostId(pub Option<PostId>);
impl From<Option<i64>> for OptPostId {
	fn from(value: Option<i64>) -> Self {
		Self(value.map(From::from))
	}
}
