use axum::{body::Body, http::Uri, response::Response};
use include_dir::{Dir, include_dir};

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

fn response_for_file(path: &str) -> Option<Response> {
    FRONTEND.get_file(path).map(|file| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
            .header("Content-Type", mime.as_ref())
            .body(Body::from(file.contents()))
            .unwrap()
    })
}

pub fn serve_static_path(path: &str) -> Response {
    let normalized = path.trim_start_matches('/').trim();
    let normalized = if normalized.is_empty() {
        "index.html"
    } else {
        normalized
    };

    if let Some(response) = response_for_file(normalized) {
        return response;
    }

    if normalized.ends_with('/') {
        let candidate = format!("{normalized}index.html");
        if let Some(response) = response_for_file(&candidate) {
            return response;
        }
    } else {
        let candidate = format!("{normalized}/index.html");
        if let Some(response) = response_for_file(&candidate) {
            return response;
        }
    }

    if !normalized.contains('.') {
        let candidate = format!("{normalized}.html");
        if let Some(response) = response_for_file(&candidate) {
            return response;
        }
    }

    response_for_file("index.html").unwrap()
}

pub async fn serve_static(uri: Uri) -> Response {
    serve_static_path(uri.path())
}
