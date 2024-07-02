#![warn(clippy::pedantic)]

mod authorized;
mod form_size_limit;
mod upload;

use lazy_static::lazy_static;
use rocket::{
    catch, catchers,
    form::{Errors, Form},
    fs::TempFile,
    http::Status,
    launch,
};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

lazy_static! {
    static ref UPLOAD_DIR: PathBuf = {
        Path::new("/home/trev/uploads")
            .canonicalize()
            .expect("upload dir not found")
    };
}
const TOKEN: &str = include_str!("../token");

/// represents a fallible [`Form`] data guard for [`TempFile`].
///
/// if the form data guard fails, this type exposes its errors (instead of the route failing)
type FallibleFormFile<'a> = Result<Form<TempFile<'a>>, Errors<'a>>;

/// response used by routes
type Response = (Status, Cow<'static, str>);

#[catch(401)]
const fn not_auth() -> &'static str {
    "failed, not authorized"
}

#[catch(404)]
const fn not_found() -> &'static str {
    "not found"
}

#[catch(400)]
const fn bad_req() -> &'static str {
    "failed, bad request"
}

#[catch(413)]
const fn too_big() -> &'static str {
    "failed, file too large"
}

#[launch]
fn rocket() -> _ {
    // initialize UPLOAD_DIR before server starts by using it here
    println!("using upload dir: {:?}", *UPLOAD_DIR);

    rocket::build()
        .attach(upload::single())
        .attach(upload::multi())
        .attach(upload::end_multi())
        .register("/", catchers![not_auth, not_found, bad_req, too_big])
}
