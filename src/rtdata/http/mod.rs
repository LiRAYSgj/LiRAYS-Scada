use axum::{
    Router,
    body::Body,
    response::Response,
    http::Uri,
    extract::State,
    routing::get,
    Json
};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, io};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use axum::serve::Listener;
use log::{error};
use crate::rtdata::server::{ServerTlsConfig, build_tls_acceptor};

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

mod static_resource;
use static_resource::{StaticResourceManager, StaticResource, StaticResourceInput};

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn error(message: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

/// Incoming adapter that performs TLS handshakes on accepted TCP connections.
struct TlsIncoming {
    listener: TcpListener,
    acceptor: TlsAcceptor,
}

impl TlsIncoming {
    fn new(listener: TcpListener, acceptor: TlsAcceptor) -> Self {
        Self { listener, acceptor }
    }
}

impl Listener for TlsIncoming {
    type Io = TlsStream<TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => match self.acceptor.accept(stream).await {
                    Ok(tls_stream) => return (tls_stream, addr),
                    Err(e) => {
                        error!("TLS handshake failed (HTTP): {e}");
                        continue;
                    }
                },
                Err(e) => {
                    error!("Error accepting HTTP connection: {e}");
                    continue;
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

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

async fn get_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>
) -> Result<Json<ApiResponse<StaticResource>>, Json<ApiResponse<()>>> {
    match manager.get_resource(id).await {
        Ok(Some(resource)) => Ok(Json(ApiResponse::success(resource))),
        Ok(None) => Err(Json(ApiResponse::error("Resource not found".to_string()))),
        Err(e) => Err(Json(ApiResponse::error(format!("Database error: {}", e)))),
    }
}

async fn get_all_resources(
    State(manager): State<Arc<StaticResourceManager>>
) -> Result<Json<ApiResponse<Vec<StaticResource>>>, Json<ApiResponse<()>>> {
    match manager.get_all_resources().await {
        Ok(resources) => Ok(Json(ApiResponse::success(resources))),
        Err(e) => Err(Json(ApiResponse::error(format!("Database error: {}", e)))),
    }
}

async fn create_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    Json(input): Json<StaticResourceInput>
) -> Result<Json<ApiResponse<StaticResource>>, Json<ApiResponse<()>>> {
    match manager.create_resource(input).await {
        Ok(resource) => Ok(Json(ApiResponse::success(resource))),
        Err(e) => Err(Json(ApiResponse::error(format!("Database error: {}", e)))),
    }
}

async fn update_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(input): Json<StaticResourceInput>
) -> Result<Json<ApiResponse<StaticResource>>, Json<ApiResponse<()>>> {
    match manager.update_resource(id, input).await {
        Ok(Some(resource)) => Ok(Json(ApiResponse::success(resource))),
        Ok(None) => Err(Json(ApiResponse::error("Resource not found".to_string()))),
        Err(e) => Err(Json(ApiResponse::error(format!("Database error: {}", e)))),
    }
}

async fn delete_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>
) -> Result<Json<ApiResponse<()>>, Json<ApiResponse<()>>> {
    match manager.delete_resource(id).await {
        Ok(deleted) => {
            if deleted {
                Ok(Json(ApiResponse::success(())))
            } else {
                Err(Json(ApiResponse::error("Resource not found".to_string())))
            }
        }
        Err(e) => Err(Json(ApiResponse::error(format!("Database error: {}", e)))),
    }
}

pub async fn run_http_server(host: &str, port: u16, db_file: &str, tls_config: Option<ServerTlsConfig>) {
    // Initialize the static resource manager
    let resource_manager = Arc::new(
        StaticResourceManager::new(db_file)
            .await
            .expect("Failed to initialize static resource manager")
    );

    let app = Router::new()
        // Static resource routes
        .route("/api/resources", get(get_all_resources).post(create_resource))
        .route("/api/resources/{id}", get(get_resource).put(update_resource).delete(delete_resource))
        // Fallback to static file serving
        .fallback(serve_static)
        // Add state to the app
        .with_state(resource_manager);

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host/port");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    if let Some(cfg) = tls_config {
        let acceptor = build_tls_acceptor(&cfg).expect("Failed to build TLS acceptor for HTTP");
        let incoming = TlsIncoming::new(listener, acceptor);
        axum::serve(incoming, app).await.expect("HTTPS server error");
    } else {
        axum::serve(listener, app).await.expect("HTTP server error");
    }
}
