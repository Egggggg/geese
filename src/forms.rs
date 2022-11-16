use rocket::fs::TempFile;

use crate::hexcolor::HexColor;

#[derive(FromForm)]
pub struct Creation {
    pub name: String,
    pub description: String,
    pub color: HexColor,
}
