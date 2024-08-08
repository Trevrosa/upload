use std::borrow::Cow;
use std::env::temp_dir;

use rocket::fairing::AdHoc;
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::tokio::fs;
use rocket::{post, routes, FromForm};
use xxhash_rust::xxh32::xxh32;

use crate::authorized::Authorized;
use crate::form_size_limit::FormSizeLimit;
use crate::{FallibleForm, Response};

#[derive(FromForm)]
struct Chunk<'a> {
    file: TempFile<'a>,
    hash: u32,
}

type FallibleFormChunk<'a> = FallibleForm<'a, Chunk<'a>>;

// `id` is the unique id used for an upload split to multiple requests
#[post("/multi/<id>/<num>", data = "<chunk>")]
async fn upload_multi(
    _size: FormSizeLimit, // check the size of upload
    _authed: Authorized,  // check the if request is authorized
    id: &'_ str,
    num: u32,
    chunk: FallibleFormChunk<'_>,
) -> Response {
    if id.ends_with(")-") {
        return (
            Status::BadRequest,
            Cow::Borrowed("id may not end with `)-`"),
        );
    }

    let mut chunk = match chunk {
        Ok(input) => input,
        Err(errs) => {
            let err = errs.first().unwrap();
            let err = format!("error occured while recieving form: {err:#?}");

            return (Status::UnprocessableEntity, Cow::Owned(err));
        }
    };

    let path = format!("({id})-{num}");
    let path = temp_dir().join(path);

    let file = &mut chunk.file;
    let save_result = file.persist_to(&path).await;

    if let Err(err) = save_result {
        let err = format!("failed to save: {err:#?}");
        eprintln!("{err}");

        (Status::InternalServerError, Cow::Owned(err))
    } else {
        let Ok(file) = fs::read(path).await else {
            return (
                Status::InternalServerError,
                Cow::Borrowed("failed to read chunk"),
            );
        };
        let hash = xxh32(&file, 0);

        if hash == chunk.hash {
            println!("finish multi upload (id: {id}, num: {num}), hash ok");
            (Status::Created, Cow::Borrowed("done"))
        } else {
            println!("finish multi upload (id: {id}, num: {num}), hash bad");
            (Status::BadRequest, Cow::Borrowed("hash mismatch, retry"))
        }
    }
}

pub fn multi() -> AdHoc {
    AdHoc::on_ignite("multi upload", |rocket| async {
        rocket.mount("/", routes![upload_multi])
    })
}
