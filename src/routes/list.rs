use rocket_db_pools::Connection;

use crate::GeeseDbConn;

pub struct SortKey(String);

impl<'a> rocket::form::FromFormField<'a> for SortKey {
    fn from_value(field: rocket::form::ValueField<'a>) -> rocket::form::Result<'a, Self> {
        let valid = ["name asc", "name desc", "time asc", "time desc"];
        let value = field.value;

        if valid.contains(&value) {
            Ok(Self(value.to_owned()))
        } else {
            let valid = valid.join(", ");
            Err(rocket::form::Error::validation(format!(
                "must be one of [{valid}]"
            )))?
        }
    }

    fn default() -> Option<Self> {
        Some(Self("name asc".to_owned()))
    }
}

#[get("/list?<sort>&<limit>&<page>")]
pub async fn list_geese<'a>(
    client: Connection<GeeseDbConn>,
    sort: SortKey,
    limit: Option<u8>,
    page: Option<u16>,
) {
}
