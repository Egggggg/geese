mod forms;
mod hexcolor;
mod models;

#[macro_use]
extern crate rocket;

use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection};
use rocket::{
    http::{ContentType, Status},
    response::content,
};
use rocket_db_pools::{Connection, Database};
use tera::{Context, Tera};

use crate::models::Goose;

const DB_NAME: &str = "geese";
const COLL_NAME: &str = "geese";

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("assets/templates/**/*.html") {
            Ok(t) => {
                println!("{:#?}", t);
                t
            }
            Err(e) => {
                eprintln!("Parsing error(s): {e}");
                std::process::exit(1);
            }
        }
    };
}

#[derive(Database)]
#[database("geese")]
pub struct GeeseDbConn(mongodb::Client);

#[get("/goose/<slug>")]
async fn get_goose(
    client: Connection<GeeseDbConn>,
    slug: &str,
) -> Result<(ContentType, String), Status> {
    let collection: Collection<Goose> = client.database(DB_NAME).collection(COLL_NAME);

    match collection.find_one(doc! { "slug": &slug }, None).await {
        Ok(Some(goose)) => {
            let mut context = Context::new();
            context.insert("goose", &goose);

            match TEMPLATES.render("goose.html", &context) {
                Ok(rendered) => Ok((ContentType::HTML, rendered)),
                Err(err) => {
                    eprintln!("{}", err);
                    Err(Status::InternalServerError)
                }
            }
        }
        Ok(None) => Err(Status::NotFound),
        Err(err) => {
            eprintln!("{}", err);
            Err(Status::InternalServerError)
        }
    }
}
#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(GeeseDbConn::init())
        .mount("/", routes![get_goose])
}
