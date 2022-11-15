use serde::{Deserialize, Serialize};

use crate::hex::Hex;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Goose {
    pub name: String,
    pub description: String,
    pub image: String,
    pub color: String,
    pub likes: i64,
    pub slug: String,
}
