mod hex;
mod models;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use lazy_static::lazy_static;
use mongodb::{bson::doc, Client, Collection};
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

#[get("/goose/{slug}")]
async fn get_goose(client: web::Data<Client>, slug: web::Path<String>) -> impl Responder {
    let slug = slug.into_inner();
    let collection: Collection<Goose> = client.database(DB_NAME).collection(COLL_NAME);

    match collection.find_one(doc! { "slug": &slug }, None).await {
        Ok(Some(goose)) => {
            let mut context = Context::new();
            context.insert("goose", &goose);

            match TEMPLATES.render("goose.html", &context) {
                Ok(rendered) => HttpResponse::Ok().body(rendered),
                Err(err) => {
                    eprintln!("{}", err);
                    HttpResponse::InternalServerError().body("Templating failed")
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body(format!("No goose with slug {slug}")),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("MONGODB_URI").unwrap();

    let client = Client::with_uri_str(uri).await.expect("failed to connect");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(get_goose)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
