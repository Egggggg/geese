use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Goose {
    pub name: String,
    pub description: String,
    pub image: String,
    pub likes: i64,
    pub slug: String,
    // pub creator: String, // profile slug
    // pub creator_name: String,
    pub timestamp: DateTime,
}
