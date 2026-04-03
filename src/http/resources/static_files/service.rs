use axum::{
    body::Body,
    http::Uri,
    response::Response,
};
use include_dir::{include_dir, Dir};

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

pub async fn serve_static(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match FRONTEND.get_file(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header("Content-Type", mime.as_ref())
                .body(Body::from(file.contents()))
                .unwrap()
        }
        None => {
            let index = FRONTEND.get_file("index.html").unwrap();
            Response::builder()
                .header("Content-Type", "text/html")
                .body(Body::from(index.contents()))
                .unwrap()
        }
    }
}
