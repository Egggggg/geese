mod cloudinary;
mod routes;
mod structures;

#[macro_use]
extern crate rocket;

use std::borrow::Cow;

use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection};
use rocket::{
    fs::{relative, FileServer},
    http::{ContentType, Status},
};
use rocket_db_pools::{Connection, Database};
use tera::{Context, Tera};

use crate::routes::{create_goose, list_geese};
use crate::structures::models::Goose;

const COLL_NAME: &str = "geese";
const TEMP_DIR: &str = "/tmp/";

#[derive(Clone, Debug)]
pub struct CloudinaryUploadConfig<'a> {
    key: Cow<'a, str>,
    secret: Cow<'a, str>,
    prefix: Cow<'a, str>,
    upload_url: Cow<'a, str>,
    fetch_url: Cow<'a, str>,
}

#[derive(Clone, Debug)]
pub struct CloudinaryFetchUrl<'a>(Cow<'a, str>);

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("assets/templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Parsing error(s): {e}");
                std::process::exit(1);
            }
        }
    };
    pub static ref DB_NAME: String = std::env::var("DB_NAME").unwrap();
}

#[derive(Database)]
#[database("geese")]
pub struct GeeseDbConn(mongodb::Client);

#[get("/goose/<slug>")]
async fn get_goose(
    client: Connection<GeeseDbConn>,
    slug: &str,
) -> Result<(ContentType, String), Status> {
    let collection: Collection<Goose> = client.database(&DB_NAME).collection(COLL_NAME);

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
    dotenv::dotenv().unwrap();

    let key: Cow<str> = Cow::Owned(std::env::var("CLOUDINARY_KEY").unwrap());
    let secret: Cow<str> = Cow::Owned(std::env::var("CLOUDINARY_SECRET").unwrap());
    let prefix: Cow<str> = Cow::Owned(std::env::var("CLOUDINARY_PREFIX").unwrap());
    let name = std::env::var("CLOUDINARY_NAME").unwrap();
    let upload_url: Cow<str> = Cow::Owned(format!(
        "https://api.cloudinary.com/v1_1/{name}/image/upload/"
    ));
    let fetch_url: Cow<str> = Cow::Owned(format!(
        "https://res.cloudinary.com/{name}/image/upload/{prefix}/"
    ));

    let fetch_url_state = CloudinaryFetchUrl(Cow::from(fetch_url.to_string()));
    let upload_config = CloudinaryUploadConfig {
        key,
        secret,
        prefix,
        upload_url,
        fetch_url,
    };

    rocket::build()
        .attach(GeeseDbConn::init())
        .manage(upload_config)
        .manage(fetch_url_state)
        .mount("/", routes![get_goose, create_goose])
        .mount("/", FileServer::from(relative!("static")))
}
