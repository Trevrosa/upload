use std::borrow::Cow;
use std::env::temp_dir;

use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::{put, routes};

use crate::authorized::Authorized;
use crate::form_size_limit::FormSizeLimit;
use crate::{FallibleFormFile, Response};

// `id` is the unique id used for an upload split to multiple requests
#[put("/multi/<id>/<num>", data = "<file>")]
async fn upload_multi(
    _size: FormSizeLimit, // check the size of upload
    _authed: Authorized,  // check the if request is authorized
    id: &'_ str,
    num: u32,
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

    let path = format!("{id}-({num})");
    let path = temp_dir().join(path);

    if path.exists() {
        println!("duplicate multi upload skipped");
        return (
            Status::Conflict,
            Cow::Borrowed("split upload already exists for this id and number"),
        );
    }

    let save_result = file.persist_to(&path).await;

    if let Err(err) = save_result {
        let err = format!("failed to save: {err:#?}");
        eprintln!("{err}");

        (Status::InternalServerError, Cow::Owned(err))
    } else {
        println!("finish multi upload (id: {id}, num: {num})");
        (Status::Created, Cow::Borrowed("done"))
    }
}

pub fn multi() -> AdHoc {
    AdHoc::on_ignite("multi upload", |rocket| async {
        rocket.mount("/", routes![upload_multi])
    })
}
