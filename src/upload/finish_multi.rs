use std::borrow::Cow;
use std::cmp::Ordering;
use std::env::temp_dir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use glob::glob;
use rocket::fairing::AdHoc;
use rocket::response::stream::{Event, EventStream};
use rocket::tokio::task::spawn_blocking;
use rocket::tokio::{fs, io};
use rocket::{get, routes};

use crate::{UPLOAD_DIR, UPLOAD_URL};

/// order two file names with template `({id})-{num}` by `num`
///
/// # Panics
///
/// will panic if the path names given do not start with a number separated with a '-'.
fn order_chunks(c1: &Path, c2: &Path) -> Ordering {
    let c1: u32 = c1
        .file_name()
        .unwrap()
        .to_string_lossy()
        .split('-')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let c2: u32 = c2
        .file_name()
        .unwrap()
        .to_string_lossy()
        .split('-')
        .next()
        .unwrap()
        .parse()
        .unwrap();

    c1.cmp(&c2)
}

enum EventIds {
    ServerError,
    IdNotFound,
    MissingChunks,
    Progress,
    Duplicate,
    Done,
}

use EventIds::{Done, Duplicate, IdNotFound, MissingChunks, Progress, ServerError};

// for use in Event::id
impl From<EventIds> for Cow<'static, str> {
    fn from(val: EventIds) -> Self {
        let id = match val {
            ServerError => "servererror",
            IdNotFound => "idnotfound",
            MissingChunks => "missingchunks",
            Progress => "progress",
            Duplicate => "duplicate",
            Done => "done",
        };

        Cow::Borrowed(id)
    }
}

// merges separated files into `name`
// this is get because js's `EventSource` sends get requests
#[get("/done/<id>/<name>/<total>")]
fn finish_multi<'a>(id: &'a str, name: &'a str, total: usize) -> EventStream![Event + 'a] {
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

        // safety:
        // `name` cannot be `..` because the url `/done/id/../total` would resolve to `/done/id/total`, thus not matched by this route
        // `name` also cannot include extra directories; the url `/done/id/etc/etc/total` would not be matched by this route
        let final_path = UPLOAD_DIR.join(name);
        let final_file = fs::File::options()
            .write(true)
            .create_new(true)
            .open(&final_path)
            .await;

        let mut final_file = match final_file {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                println!("duplicate file {final_path:?} skipped combination, deleting temp files");
                for file in files {
                    if fs::remove_file(&file).await.is_err() {
                        eprintln!("failed to delete temp file {file:?}");
                    };
                }

                yield Event::data("cannot upload because duplicate").id(Duplicate);
                return;
            }
            Err(err) => {
                let err = format!("error occured while creating file {err}");
                eprintln!("{err}");

                yield Event::data(err).id(ServerError);
                return;
            }
        };

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
