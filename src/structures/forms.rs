use rocket::fs::TempFile;

use super::hexcolor::HexColor;

#[derive(FromForm)]
pub struct Creation<'a> {
    pub name: String,
    pub description: String,
    pub color: HexColor,
    pub image: TempFile<'a>,
}
