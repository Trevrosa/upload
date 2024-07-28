use std::env::temp_dir;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;

use glob::glob;
use rocket::fairing::AdHoc;
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::task::spawn_blocking;
use rocket::tokio::{fs, io};
use rocket::{get, routes};

use crate::{UPLOAD_DIR, UPLOAD_URL};

// merges separated files into `name`
// this is `get` because js's `EventSource` sends `get` requests
#[allow(clippy::needless_pass_by_value)]
#[get("/done/<id>/<name>/<total>")]
fn finish_multi<'a>(id: &'a str, name: &'a str, total: usize) -> EventStream![Event + 'a] {
    let stream = EventStream! {
        let matcher = temp_dir().join(format!("{id}*"));

        let files = spawn_blocking(move || glob(matcher.to_str().unwrap()).unwrap());
        // ok not to sort because glob already sorts alphabetically
        let files: Vec<Result<PathBuf, _>> = files.await.unwrap().collect();

        if files.is_empty() {
            yield Event::data("file not found from id").id("idnotfound");
            return;
        }

        if files.len() != total {
            let msg = format!(
                "{}/{total} uploads received, upload the missing chunks and retry",
                files.len()
            );

            yield Event::data(msg).id("missingchunks");
            return;
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

                yield Event::data("cannot upload because duplicate").id("duplicate");
                return;
            }
            Err(err) => {
                let err = format!("error occured while creating file {err}");
                eprintln!("{err}");

                yield Event::data(err).id("servererror");
                return;
            }
        };

        for (n, path) in files.iter().enumerate() {
            let path = path.as_ref().unwrap();
            let mut file = fs::File::open(&path).await.unwrap();

            if let Err(err) = io::copy(&mut file, &mut final_file).await {
                eprintln!("\nerror occurred while merging file {:?} ({err:?})", &path);
                yield Event::data(format!("merge file error: {err:#?}")).id("servererror");
                return;
            };

            if let Err(err) = fs::remove_file(path).await {
                eprintln!("\nfailed to delete old file ({err:?})");
            }

            println!("combined file (id: {id}, num: {})", n + 1);
        }

        println!("finish combine upload (id: {id}) to {final_path:?}");

        let url = format!("{UPLOAD_URL}/{name}");
        yield Event::data(url).id("done");
    };

    stream.heartbeat(Duration::from_secs(15))
}

pub fn end_multi() -> AdHoc {
    AdHoc::on_ignite("finish multi upload", |rocket| async {
        rocket.mount("/", routes![finish_multi])
    })
}
