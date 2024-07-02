use std::borrow::Cow;
use std::env::temp_dir;
use std::io::ErrorKind;
use std::path::PathBuf;

use glob::glob;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::tokio::task::spawn_blocking;
use rocket::tokio::{fs, io};
use rocket::{put, routes};

use crate::authorized::Authorized;
use crate::{Response, UPLOAD_DIR};

// merges separated files into `name`
#[put("/done/<id>/<name>")]
async fn finish_multi(
    _token: Authorized, // check if request is authorized
    id: &'_ str,
    name: &'_ str,
) -> Response {
    let matcher = temp_dir().join(format!("{id}*"));

    let files = spawn_blocking(move || glob(matcher.to_str().unwrap()).unwrap());
    let files: Vec<Result<PathBuf, _>> = files.await.unwrap().collect();

    if files.is_empty() {
        return (Status::NotFound, Cow::Borrowed("file not found from id"));
    }

    let final_path = UPLOAD_DIR.join(name);
    let final_file = fs::File::options()
        .write(true)
        .truncate(true)
        .create_new(true)
        .open(&final_path)
        .await;

    let mut final_file = match final_file {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            println!("duplicate file {final_path:?} skipped combination, deleting temp files");
            for file in files {
                let file = file.unwrap();
                if fs::remove_file(&file).await.is_err() {
                    eprintln!("failed to delete temp file {file:?}");
                };
            }

            return (
                Status::Conflict,
                Cow::Borrowed("cannot upload because duplicate"),
            );
        }
        Err(err) => {
            eprintln!("error occured while creating file {err}",);
            return (
                Status::InternalServerError,
                Cow::Borrowed("failed to create file"),
            );
        }
    };

    for (n, path) in files.iter().enumerate() {
        let path = path.as_ref().unwrap();
        let mut file = fs::File::open(&path).await.unwrap();

        if let Err(err) = io::copy(&mut file, &mut final_file).await {
            eprintln!("\nerror occurred while merging file {:?} ({err:?})", &path);
            return (
                Status::InternalServerError,
                Cow::Owned(format!("merge file error: {err:#?}")),
            );
        };

        if let Err(err) = fs::remove_file(path).await {
            eprintln!("\nfailed to delete old file ({err:?})");
        }

        println!("combined file (id: {id}, num: {})", n + 1);
    }

    println!("finish combine upload (id: {id}) to {final_path:?}");

    let url = format!("https://uploads.trevrosa.dev/{name}");
    (Status::Created, Cow::Owned(url))
}

pub fn end_multi() -> AdHoc {
    AdHoc::on_ignite("finish multi upload", |rocket| async {
        rocket.mount("/", routes![finish_multi])
    })
}
