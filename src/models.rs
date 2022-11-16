use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Goose {
    pub name: String,
    pub description: String,
    pub image: String,
    pub color: String,
    pub likes: i64,
    pub slug: String,
    // pub creator: ObjectId,
    // pub timestamp: DateTime,
}
