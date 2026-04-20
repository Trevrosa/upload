use std::borrow::Cow;
use std::env::temp_dir;
use std::io::SeekFrom;

use rocket::fairing::AdHoc;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::tokio::io::{AsyncReadExt, AsyncSeekExt};
use rocket::tokio::{fs, io};
use rocket::{FromForm, post, routes};
use xxhash_rust::xxh32::xxh32;

use crate::authorized::Authorized;
use crate::form_size_limit::FormSizeLimit;
use crate::{FallibleForm, Response};

#[derive(FromForm)]
struct Chunk<'a> {
    num: u64,
    file: TempFile<'a>,
    hash: u32,
}

type FallibleFormChunk<'a> = FallibleForm<'a, Chunk<'a>>;

// `id` is the unique id used for an upload split to multiple requests
#[post("/multi/<id>/<total>", data = "<chunk>")]
async fn upload_multi(
    _size: FormSizeLimit, // check the size of upload
    _authed: Authorized,  // check the if request is authorized
    id: &'_ str,
    total: u64,
    chunk: FallibleFormChunk<'_>,
) -> Response {
    let mut chunk = match chunk {
        Ok(input) => input,
        Err(errs) => {
            let err = errs.first().unwrap();
            let err = format!("error occured while recieving form: {err:#?}");

            return (Status::UnprocessableEntity, Cow::Owned(err));
        }
    };

    let num = chunk.num;
    let hash = chunk.hash;
    let upload = &mut chunk.file;

    let file = fs::File::options()
        .write(true)
        .open(temp_dir().join(id))
        .await;
    match file {
        Ok(mut file) => {
            let Ok(mut read) = upload.open().await else {
                return (
                    Status::InternalServerError,
                    Cow::Borrowed("failed to read chunk"),
                );
            };

            let mut bytes = Vec::new();
            read.read_to_end(&mut bytes).await.unwrap();

            if xxh32(&bytes, 0) != hash {
                println!("chunk hash mismatch (id: {id}, num: {num})");
                return (Status::BadRequest, Cow::Borrowed("hash mismatch, retry"));
            }

            if let Err(err) = file.seek(SeekFrom::Start(num)).await {
                eprintln!("could not seek file: {err}");
                return (Status::InternalServerError, Cow::Borrowed("io error"));
            };

            if let Err(err) = io::copy(&mut read, &mut file).await {
                eprintln!("could not copy bytes to tempfile: {err}");
                return (
                    Status::InternalServerError,
                    Cow::Borrowed("could not write file"),
                );
            }

            if let Some(upload) = upload.path()
                && let Err(err) = fs::remove_file(upload).await
            {
                eprintln!("failed to cleanup uploaded file: {err}");
            }

            println!("chunk received (id: {id}, num: {num})");
            (Status::Created, Cow::Borrowed("done"))
        }
        Err(err) => {
            eprintln!("could not open file: {err}");

            (
                Status::InternalServerError,
                Cow::Borrowed("could not open file"),
            )
        }
    }
}

pub fn multi() -> AdHoc {
    AdHoc::on_ignite("multi upload", |rocket| async {
        rocket.mount("/", routes![upload_multi])
    })
}
