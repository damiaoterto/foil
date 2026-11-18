use foil_packager::{FoilArchiveReader, PRODUCTION_CSP};
use std::borrow::Cow;
use wry::http::header::{CONTENT_SECURITY_POLICY, CONTENT_TYPE};
use wry::http::{Request, Response, StatusCode};

pub fn handle_foil_protocol_request(
    archive: &FoilArchiveReader,
    request: &Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let uri_path = request.uri().path();
    match archive.get_asset(uri_path) {
        Some((bytes, mime)) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, mime)
            .header(CONTENT_SECURITY_POLICY, PRODUCTION_CSP)
            .header("X-Content-Type-Options", "nosniff")
            .header("X-Frame-Options", "DENY")
            .body(Cow::Owned(bytes.to_vec()))
            .unwrap_or_else(|_| empty_error_response(StatusCode::INTERNAL_SERVER_ERROR)),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(CONTENT_SECURITY_POLICY, PRODUCTION_CSP)
            .body(Cow::Borrowed(b"Not Found" as &[u8]))
            .unwrap_or_else(|_| empty_error_response(StatusCode::NOT_FOUND)),
    }
}

fn empty_error_response(status: StatusCode) -> Response<Cow<'static, [u8]>> {
    let mut response = Response::new(Cow::Borrowed(b"" as &[u8]));
    *response.status_mut() = status;
    response
}
