mod cloudinary;
mod forms;
mod hexcolor;
mod models;

#[macro_use]
extern crate rocket;

use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rocket::{
    form::Form,
    fs::{relative, FileServer},
    http::{ContentType, Status},
    response::Redirect,
};
use rocket_db_pools::{Connection, Database};
use tera::{Context, Tera};

use crate::models::Goose;

const COLL_NAME: &str = "geese";
const TEMP_DIR: &str = "/tmp/";

// change these if you're reusing this
const CLOUDINARY_URL: &str = "https://res.cloudinary.com/beesbeesbees/image/upload/geese/";
const CLOUDINARY_UPLOAD: &str = "https://api.cloudinary.com/v1_1/beesbeesbees/image/upload/";

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
    pub static ref CLOUDINARY_KEY: String = std::env::var("CLOUDINARY_KEY").unwrap();
    pub static ref CLOUDINARY_SECRET: String = std::env::var("CLOUDINARY_SECRET").unwrap();
    pub static ref CLOUDINARY_PREFIX: String = std::env::var("CLOUDINARY_PREFIX").unwrap();
}

#[derive(Database)]
#[database("geese")]
pub struct GeeseDbConn(mongodb::Client);

#[derive(Responder)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

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

#[post("/goose", data = "<input>")]
async fn create_goose(
    client: Connection<GeeseDbConn>,
    mut input: Form<forms::Creation<'_>>,
) -> Result<Either<Redirect, (ContentType, String)>, (Status, &'static str)> {
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
    let collection: Collection<Goose> = client.database(&DB_NAME).collection(COLL_NAME);

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
                // try to get the path to the temp file
                // if it's not fully in the filesystem, this returns None, so move it into the filesystem and set path to that path
                let (path, persisted) = match input.image.path() {
                    Some(path) => (path, false),
                    None => {
                        let path = TEMP_DIR.to_owned() + &slug;

                        // move the temp file from memory to an actual file so we can open it
                        input.image.persist_to(&path).await.map_err(|e| {
                            eprintln!("Error while persisting {path}: {e}");
                            (
                                Status::InternalServerError,
                                "Encountered an error while persisting upload",
                            )
                        })?;

                        (input.image.path().unwrap(), true)
                    }
                };

                cloudinary::upload(path, &slug).await?;

                if persisted {
                    tokio::fs::remove_file(path).await.unwrap_or_default();
                }

                // url of uploaded goose
                let image = CLOUDINARY_URL.to_owned() + &slug;

                // put the goose in the database
                let goose = Goose {
                    name: (&input.name).to_owned(),
                    description: (&input.description).to_owned(),
                    color: input.color.inner().to_owned(),
                    slug: slug.clone(),
                    likes: 0,
                    image,
                };

                return match collection.insert_one(goose, None).await {
                    Ok(_) => Ok(Either::Left(Redirect::to(uri!(get_goose(slug))))),
                    Err(e) => {
                        eprintln!("Error creating goose: {e}");

                        Err((
                            Status::InternalServerError,
                            "We couldn't create your goose :(",
                        ))
                    }
                };
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
    dotenv::dotenv().unwrap();

    rocket::build()
        .attach(GeeseDbConn::init())
        .mount("/", routes![get_goose, create_goose])
        .mount("/", FileServer::from(relative!("static")))
}
