use std::borrow::Cow;
use std::cmp::Ordering;
use std::env::temp_dir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use glob::glob;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::task::spawn_blocking;
use rocket::tokio::{fs, io};
use rocket::{get, routes};

use crate::{Response, UPLOAD_DIR, UPLOAD_URL};

// merges separated files into `name`
#[get("/done/<id>/<name>/<total_size>")]
async fn finish_multi<'a>(id: &'a str, name: &'a str, total_size: u64) -> Response {
    let path = temp_dir().join(id);

    let meta = {
        let Ok(file) = fs::File::open(&path).await else {
            return (Status::NotFound, Cow::Borrowed("no file with provided id"))
        };
        
        let Ok(meta) = file.metadata().await else {
            return (Status::InternalServerError, Cow::Borrowed("could not read file with provided id"))
        };

        meta
    };

    if meta.len() != total_size {
        return (Status::BadRequest, Cow::Borrowed("size mismatch"))
    }

    // safety:
    // `name` cannot be `..` because the url `/done/id/../total` would resolve to `/done/id/total`, thus not matched by this route
    // `name` also cannot include extra directories; the url `/done/id/etc/etc/total` would not be matched by this route
    let final_path = UPLOAD_DIR.join(name);

    if fs::try_exists(&final_path).await.is_ok_and(|exists| exists) {
        println!("duplicate file {final_path:?}, deleting temp file");
        if let Err(err) = fs::remove_file(&path).await {
            
        }
        return (Status::Conflict, Cow::Borrowed("target file already exists"))        
    }

    if let Err(err) = fs::rename(from, to)

    let mut final_file = match final_file {
        Ok(file) => file,
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {

        }
        Err(err) => {
            
        }
    };

    let stream = EventStream! {
        let matcher = temp_dir().join(format!("*{id}"));

        let files = spawn_blocking(move || glob(matcher.to_str().unwrap()).unwrap()).await;
        let Ok(files) = files else {
            yield Event::data("failed to find chunks").id(ServerError);
            return;
        };

        // unwrap but ignore Err variants
        let mut files: Vec<PathBuf> = files.flatten().collect();

        if files.len() != total && !files.is_empty() {
            let msg = format!(
                "{} chunks were received, but {total} chunks was specified. are some chunks missing?",
                files.len()
            );

            yield Event::data(msg).id(MissingChunks);
            return;
        }

        if files.is_empty() {
            yield Event::data("file not found from id").id(IdNotFound);
            return;
        }

        if files.iter().any(|e| !e.to_string_lossy().contains('-')) {
            yield Event::data("one or more chunks not saved correctly").id(ServerError);
            return;
        }
        files.sort_by(|a, b| order_chunks(a, b));

        

        for (n, path) in files.iter().enumerate() {
            let Ok(mut file) = fs::File::open(&path).await else {
                yield Event::data(format!("failed to read chunk #{n}")).id(ServerError);
                return;
            };

            if let Err(err) = io::copy(&mut file, &mut final_file).await {
                eprintln!("error occurred while merging file {:?} ({err:?})", &path);
                yield Event::data(format!("merge file error: {err:#?}")).id(ServerError);
                return;
            };

            if let Err(err) = fs::remove_file(path).await {
                eprintln!("failed to delete old file ({err:?})");
            }

            println!("combined file (id: {id}, num: {})", n + 1);
            yield Event::data((n + 1).to_string()).id(Progress);
        }

        println!("finish combine upload (id: {id}) to {final_path:?}");

        let url = format!("{UPLOAD_URL}/{name}");
        yield Event::data(url).id(Done);
    };

    stream.heartbeat(Duration::from_secs(15))
}

pub fn end_multi() -> AdHoc {
    AdHoc::on_ignite("finish multi upload", |rocket| async {
        rocket.mount("/", routes![finish_multi])
    })
}
