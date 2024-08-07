#![warn(clippy::pedantic)]

mod authorized;
mod form_size_limit;
mod upload;

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
    sync::LazyLock,
};

static UPLOAD_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(include_str!("../upload_path"))
        .canonicalize()
        .expect("upload dir not found")
});

const UPLOAD_URL: &str = include_str!("../upload_url");
const TOKEN: &str = include_str!("../token");

/// fallible [`Form`] data guard for `F`
type FallibleForm<'a, F> = Result<Form<F>, Errors<'a>>;

/// represents a fallible [`Form`] data guard for [`TempFile`].
///
/// if the form data guard fails, this type exposes its errors (instead of the route failing)
type FallibleFormFile<'a> = FallibleForm<'a, TempFile<'a>>;

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
