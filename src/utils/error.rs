use rocket::http::{ContentType, Status};
use rocket::response::{Responder, Response, Result};
use rocket::serde::json::json;
use rocket::Request;
use std::io::Cursor;

#[derive(Debug)]
pub struct ApiError {
    pub status: Status,
    pub message: String,
}

impl ApiError {
    pub fn internal(msg: &str) -> Self {
        Self {
            status: Status::InternalServerError,
            message: msg.to_string(),
        }
    }

    pub fn bad_request(msg: &str) -> Self {
        Self {
            status: Status::BadRequest,
            message: msg.to_string(),
        }
    }

    pub fn unauthorized(msg: &str) -> Self {
        Self {
            status: Status::Unauthorized,
            message: msg.to_string(),
        }
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> Result<'static> {
        let body = serde_json::to_string(&json!({
            "status": "error",
            "message": self.message
        }))
        .unwrap();

        return Response::build()
            .status(self.status)
            .header(ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok();
    }
}
