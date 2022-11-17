use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::multipart::Part;
use rocket::http::Status;
use sha1::{Digest, Sha1};

use crate::{CLOUDINARY_KEY, CLOUDINARY_PREFIX, CLOUDINARY_SECRET, CLOUDINARY_UPLOAD};

pub async fn upload(path: &Path, slug: &str) -> Result<(), (Status, &'static str)> {
    let public_id = CLOUDINARY_PREFIX.to_string() + &slug;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            eprintln!("Time machine don't work: {e}");

            (
                Status::InternalServerError,
                "Something went wrong. Please try again in a few moments",
            )
        })?
        .as_secs()
        .to_string();

    let signature =
        "public_id=".to_owned() + &public_id + "&timestamp=" + &timestamp + &CLOUDINARY_SECRET;

    println!("{}", signature);

    let mut hasher = Sha1::new();
    hasher.update(signature.as_bytes());
    let signature = hasher.finalize();
    let signature = hex::encode(signature);

    println!("{}", signature);

    let file = tokio::fs::File::open(path).await.map_err(|_| {
        (
            Status::InternalServerError,
            "Failed to upload goose image. Please try again in a few moments",
        )
    })?;

    let form = reqwest::multipart::Form::new()
        .text("api_key", CLOUDINARY_KEY.as_str())
        .text("timestamp", timestamp)
        .text("signature", signature)
        .text("public_id", public_id)
        .part("file", Part::stream(file).file_name(slug.to_owned()));

    let client = reqwest::Client::new();
    let res = client
        .post(CLOUDINARY_UPLOAD)
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error uploading to cloudinary: {e}");

            (
                Status::InternalServerError,
                "Failed to upload goose image. Please try again in a few moments",
            )
        })?;

    println!("{:#?}", res);

    Ok(())
}
