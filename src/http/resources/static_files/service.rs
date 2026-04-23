use axum::{body::Body, http::Uri, response::Response};
use include_dir::{Dir, include_dir};
use log::warn;

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

fn response_for_file(path: &str) -> Option<Response> {
    FRONTEND.get_file(path).and_then(|file| {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
            .header("Content-Type", mime.as_ref())
            .body(Body::from(file.contents()))
            .map_err(|e| {
                warn!("Failed building static response for '{path}': {e}");
                e
            })
            .ok()
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

    // For missing concrete assets (e.g. .js/.css/.png), do not fall back to index.html.
    // Returning HTML for module script requests causes MIME errors and blank screens.
    if normalized.contains('.') {
        return Response::builder()
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Body::from("Not Found"))
            .unwrap_or_else(|_| Response::new(Body::from("Not Found")));
    }

    response_for_file("index.html").unwrap_or_else(|| {
        Response::builder()
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Body::from("Not Found"))
            .unwrap_or_else(|_| Response::new(Body::from("Not Found")))
    })
}

pub async fn serve_static(uri: Uri) -> Response {
    serve_static_path(uri.path())
}
