mod forms;
mod hexcolor;
mod models;

#[macro_use]
extern crate rocket;

use std::io;

use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rocket::{
    form::Form,
    fs::NamedFile,
    http::{ContentType, Status},
    response::stream::{Event, EventStream},
};
use rocket_db_pools::{Connection, Database};
use tera::{Context, Tera};

use crate::models::Goose;

const DB_NAME: &str = "geese";
const COLL_NAME: &str = "geese";
const CLOUDINARY_URL: &str =
    "https://res.cloudinary.com/beesbeesbees/image/upload/w_512,h_512/geese/";

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

#[get("/hatchery")]
async fn hatchery() -> io::Result<NamedFile> {
    NamedFile::open("/home/bee/dev/geese/assets/static/upload.html").await
}

#[post("/goose", data = "<input>")]
async fn create_goose(
    client: Connection<GeeseDbConn>,
    input: Form<forms::Creation>,
) -> Result<EventStream![], (Status, &'static str)> {
    // sluggify name
    // convert spaces into hyphens, then keep only hyphens and alphanumeric characters
    let mut slug: String = input
        .name
        .clone()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .filter(|c| c == &'-' || c.is_ascii_alphanumeric())
        .collect();

    if slug.len() == 0 {
        slug = String::from_utf8(thread_rng().sample_iter(&Alphanumeric).take(5).collect())
            .map_err(|e| {
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
            Ok(Some(_)) => {
                // theres already a goose with this slug, get ready to retry
                // generate a random alphanumeric character to append to slug
                let chosen: String =
                    String::from_utf8(thread_rng().sample_iter(&Alphanumeric).take(1).collect())
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
                // return a single use event stream to tell the user which slug to use
                return Ok(EventStream! {
                    // send the slug
                    yield Event::data(slug.clone());

                    // url of uploaded goose
                    let image = CLOUDINARY_URL.to_owned() + &slug;

                    // put the goose in the database
                    let goose = Goose {
                        name: (&input.name).to_owned(),
                        description: (&input.description).to_owned(),
                        color: input.color.inner().to_owned(),
                        slug,
                        likes: 0,
                        image,
                    };

                    match collection.insert_one(goose, None).await {
                        Ok(_) => yield Event::data("created"),
                        Err(e) => {
                            eprintln!("Error creating goose: {e}");

                            yield Event::data("failed")
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error finding document with slug {slug}: {e}");
                return Err((
                    Status::InternalServerError,
                    "Please wait a few moments and try again",
                ));
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
        .mount("/", routes![get_goose, create_goose, hatchery])
}
