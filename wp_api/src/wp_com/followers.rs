use crate::users::UserId;
use serde::{Deserialize, Serialize};
use wp_serde_helper::deserialize_i64_or_string_as_t;

#[derive(Debug, Serialize, Deserialize, uniffi::Record)]
pub struct DeleteFollowerResponse {
    #[serde(deserialize_with = "deserialize_i64_or_string_as_t")]
    follower_id: UserId,
    deleted: bool,
}
