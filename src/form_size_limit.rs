use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

const LIMIT: u64 = 15_000_000;

/// request guard checking the `content-length` header
pub struct FormSizeLimit(());

#[rocket::async_trait]
impl<'r> FromRequest<'r> for FormSizeLimit {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let size = req.headers().get_one("content-length");

        #[allow(clippy::option_if_let_else)]
        match size {
            Some(size) => {
                let Ok(size) = size.parse::<u64>() else {
                    return Outcome::Error((Status::BadRequest, "file size is not a valid number"));
                };

                if size > LIMIT {
                    Outcome::Error((Status::PayloadTooLarge, "file too large"))
                } else {
                    Outcome::Success(Self(()))
                }
            }
            None => Outcome::Error((Status::BadRequest, "file size not found")),
        }
    }
}
