use axum::{Router, body::Body, response::Response, http::Uri};
use include_dir::{include_dir, Dir};

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

async fn serve_static(uri: Uri) -> Response {
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

pub async fn run_http_server(host: &str, port: u16) {
    let app = Router::new()
        .fallback(serve_static);

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host/port");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}
