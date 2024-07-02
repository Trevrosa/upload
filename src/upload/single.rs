use std::borrow::Cow;

use crate::authorized::Authorized;
use crate::form_size_limit::FormSizeLimit;
use crate::{FallibleFormFile, Response, UPLOAD_DIR};

use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::tokio::fs;
use rocket::{put, routes};

use humansize::{format_size, DECIMAL};

// this is `put` and not `post` because this function is idempotent;
// this function will not do anything if the file already exists.
#[put("/", data = "<file>")]
async fn upload_single(
    _size: FormSizeLimit, // check the size of upload
    _authed: Authorized,  // check the if request is authorized
    file: FallibleFormFile<'_>,
) -> Response {
    let mut file = match file {
        Ok(input) => input,
        Err(errs) => {
            let err = errs.first().unwrap();
            let err = format!("error occured while recieving form: {err:#?}");

            return (Status::UnprocessableEntity, Cow::Owned(err));
        }
    };

    let Some(name) = file.raw_name() else {
        return (
            Status::UnprocessableEntity,
            Cow::Borrowed("failed to get file name"),
        );
    };

    // OK because server is restricted and name is checked
    let name = name.dangerous_unsafe_unsanitized_raw().as_str();

    if name.contains("..") {
        return (
            Status::Forbidden,
            Cow::Borrowed("cannot upload because file name cannot contain `..`"),
        );
    }

    let path = UPLOAD_DIR.join(name);

    if path.exists() {
        println!("duplicate file {path:?} skipped upload");
        return (
            Status::Conflict,
            Cow::Borrowed("cannot upload because file exists already"),
        );
    }

    // humanized file size (eg. 1000 bytes => 1 kb)
    let file_size = format_size(file.len(), DECIMAL);

    // i want copy_to instead of persist_to because i need to save files in a different device
    let save_file = file.copy_to(path.clone()).await;

    if let Err(err) = save_file {
        eprintln!("failed save file to {path:?} ({err:?})");
        let err = format!("error occured while saving file: {err:#?}");

        (Status::InternalServerError, Cow::Owned(err))
    } else {
        println!("finish save file to {path:?} (size: {file_size})");

        // delete temp file if it exists
        if let Some(temp) = file.path() {
            if fs::remove_file(temp).await.is_err() {
                eprintln!("failed to delete temp file {temp:?}");
            };
        }

        // unwraps are safe because path to file should never end with `..` or be `/`
        let file = path.file_name().unwrap().to_str().unwrap();
        let url = format!("https://uploads.trevrosa.dev/{file}");

        (Status::Created, Cow::Owned(url))
    }
}

pub fn single() -> AdHoc {
    AdHoc::on_ignite("single upload", |rocket| async {
        rocket.mount("/", routes![upload_single])
    })
}
