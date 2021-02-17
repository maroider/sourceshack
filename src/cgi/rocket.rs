use std::io::Cursor;

use rocket::{http::Status, Response};

use super::CgiResponse;

impl<'r> Into<Response<'r>> for CgiResponse {
    fn into(self) -> Response<'r> {
        let mut response = Response::new();
        response.set_status(Status::raw(self.status_code));
        for (header_name, header_value) in self.headers {
            response.adjoin_raw_header(header_name, header_value);
        }
        response.set_sized_body(None, Cursor::new(self.body));
        response
    }
}
