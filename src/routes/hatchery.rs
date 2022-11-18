use crate::rocket_uri_macro_get_goose;
use mongodb::{
    bson::{doc, DateTime},
    Collection,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rocket::State;
use rocket::{
    form::Form,
    http::{ContentType, Status},
    response::Redirect,
    Either,
};
use rocket_db_pools::Connection;

use crate::{
    cloudinary, structures::forms::Creation, structures::models::Goose, CloudinaryUploadConfig,
    GeeseDbConn, COLL_NAME, DB_NAME, TEMP_DIR,
};

#[post("/goose", data = "<input>")]
pub async fn create_goose<'a>(
    client: Connection<GeeseDbConn>,
    cloudinary: &State<CloudinaryUploadConfig<'a>>,
    mut input: Form<Creation<'_>>,
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

                cloudinary::upload(&cloudinary, path, &slug).await?;

                if persisted {
                    tokio::fs::remove_file(path).await.unwrap_or_default();
                }

                // url of uploaded goose
                let image = cloudinary.fetch_url.to_string() + &slug;
                let timestamp: DateTime = DateTime::now();

                // put the goose in the database
                let goose = Goose {
                    name: (&input.name).to_owned(),
                    description: (&input.description).to_owned(),
                    slug: slug.clone(),
                    likes: 0,
                    image,
                    timestamp,
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
