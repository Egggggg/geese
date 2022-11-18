use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::multipart::Part;
use rocket::http::Status;
use sha1::{Digest, Sha1};

use crate::CloudinaryUploadConfig;

pub async fn upload(
    cloudinary: &CloudinaryUploadConfig<'_>,
    path: &Path,
    slug: &str,
) -> Result<(), (Status, &'static str)> {
    let public_id = cloudinary.prefix.to_string() + &slug;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            eprintln!("Time machine broke: {e}");

            (
                Status::InternalServerError,
                "Something went wrong. Please try again in a few moments",
            )
        })?
        .as_secs()
        .to_string();

    let signature =
        "public_id=".to_owned() + &public_id + "&timestamp=" + &timestamp + &cloudinary.secret;

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
        .text("api_key", cloudinary.key.to_string())
        .text("timestamp", timestamp)
        .text("signature", signature)
        .text("public_id", public_id)
        .part("file", Part::stream(file).file_name(slug.to_owned()));

    let client = reqwest::Client::new();
    let res = client
        .post(cloudinary.upload_url.to_string())
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

    println!("res: {:#?}", res);

    res.error_for_status().map_err(|e| {
        eprintln!("Error uploading to cloudinary: {e}");

        (
            Status::InternalServerError,
            "Failed to upload goose image. Please try again in a few moments",
        )
    })?;

    Ok(())
}
