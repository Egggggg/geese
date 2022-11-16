mod forms;
mod hexcolor;
mod models;

#[macro_use]
extern crate rocket;

use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection};
use rand::{distributions::Alphanumeric, random, thread_rng, Rng};
use rocket::{
    form::Form,
    fs::NamedFile,
    futures::io::BufReader,
    http::{ContentType, Status},
    response::Redirect,
};
use rocket_db_pools::{Connection, Database};
use std::path::Path;
use tera::{Context, Tera};

use crate::models::Goose;

const DB_NAME: &str = "geese";
const COLL_NAME: &str = "geese";
const TEMP_DIR: String = "/geese/temp/".to_owned();

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("static/templates/**/*.html") {
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

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

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

#[post("/goose", data = "<input>")]
async fn create_goose(
    client: Connection<GeeseDbConn>,
    input: Form<forms::Creation<'_>>,
) -> Result<Either<Redirect, (ContentType, String)>, (Status, &str)> {
    // sluggify name
    // convert spaces into hyphens, then keep only hyphens and alphanumeric characters
    let mut slug: String = input
        .name
        .clone()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .filter(|c| c == &'-' || c.is_ascii_alphanumeric())
        .collect();

    let mut rng = thread_rng();

    if slug.len() == 0 {
        slug = String::from_utf8(rng.sample_iter(&Alphanumeric).take(5).collect()).map_err(|e| {
            eprintln!("Error while creating slug: {e}");
            (
                Status::InternalServerError,
                "Please wait a few moments and try again",
            )
        })?
    }

    // get db collection with the geese
    let collection: Collection<Goose> = client.database(DB_NAME).collection(COLL_NAME);

    'outer: for _ in 0..5 {
        match collection.find_one(doc! { "slug": &slug }, None).await {
            Ok(Some(goose)) => {
                // theres already a goose with this slug, get ready to retry
                // generate a random alphanumeric character to append to slug
                let chosen: String = String::from_utf8(
                    rng.sample_iter(&Alphanumeric).take(1).collect(),
                )
                .map_err(|e| {
                    eprintln!("Error while extending slug: {e}");
                    (
                        Status::InternalServerError,
                        "Please wait a few moments and try again",
                    )
                })?;

                // extend slug, so it's different next time
                slug += &chosen;

                continue 'outer;
            }
            Ok(None) => {
                // there is no goose with this slug, use it
                // try to get the path to the temp file
                // if it's not fully in the filesystem, this returns None, so move it into the filesystem and set path ot that path
                let path = match input.image.path() {
                    Some(path) => path,
                    None => {
                        let path = TEMP_DIR + &slug;

                        // move the temp file from memory to an actual file so we can open it
                        input.image.persist_to(path).await.map_err(|e| {
                            eprintln!("Error while persisting {path}: {e}");
                            (
                                Status::InternalServerError,
                                "Encountered an error while persisting upload",
                            )
                        })?;

                        Path::new(&path)
                    }
                };

                let goose = Goose {
                    name: input.name,
                    description: input.description,
                    color,
                    slug,
                };
            }
        }
    }

    // failed to get a valid slug in 5 attempts
    eprintln!("No valid slugs were derived from {slug}");
    Err((Status::InternalServerError, "Failed to generate a slug"))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(GeeseDbConn::init())
        .mount("/", routes![get_goose])
}
