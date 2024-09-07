use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::TOKEN;

/// request guard checking a header named `token`
pub struct Authorized(());

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorized {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = req.headers().get_one("token");

        match token {
            Some(token) => {
                if token == TOKEN {
                    Outcome::Success(Self(()))
                } else {
                    Outcome::Error((Status::Unauthorized, "failed, not authorized"))
                }
            }
            None => Outcome::Error((Status::BadRequest, "token was not found")),
        }
    }
}
