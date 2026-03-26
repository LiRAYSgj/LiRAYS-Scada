mod model;

use std::{io, sync::Arc};

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    http::Uri,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use axum::serve::Listener;
use include_dir::{include_dir, Dir};
use log::error;
use model::resource::service::{
    StaticResource, StaticResourceInput, StaticResourceManager,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use utoipa::{OpenApi, ToSchema};

use super::tls::{build_tls_acceptor, ServerTlsConfig};

static FRONTEND: Dir = include_dir!("$CARGO_MANIFEST_DIR/frontend/build");

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiResponseResource {
    success: bool,
    data: Option<StaticResource>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiResponseResourceList {
    success: bool,
    data: Option<Vec<StaticResource>>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
struct ApiResponseEmpty {
    success: bool,
    data: Option<()>,
    message: Option<String>,
}

impl ApiResponseEmpty {
    fn success() -> Self {
        Self {
            success: true,
            data: Some(()),
            message: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
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

#[utoipa::path(
    get,
    path = "/api/resources/{id}",
    responses(
        (status = 200, description = "Resource found", body = ApiResponseResource),
        (status = 404, description = "Resource not found", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
async fn get_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    match manager.get_resource(id).await {
        Ok(Some(resource)) => (StatusCode::OK, Json(ApiResponse::success(resource))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    get,
    path = "/api/resources",
    responses(
        (status = 200, description = "List resources", body = ApiResponseResourceList)
    )
)]
async fn get_all_resources(
    State(manager): State<Arc<StaticResourceManager>>,
) -> impl IntoResponse {
    match manager.get_all_resources().await {
        Ok(resources) => (StatusCode::OK, Json(ApiResponse::success(resources))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/resources",
    request_body = StaticResourceInput,
    responses(
        (status = 200, description = "Resource created", body = ApiResponseResource),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    )
)]
async fn create_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match manager.create_resource(input).await {
        Ok(resource) => (StatusCode::OK, Json(ApiResponse::success(resource))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    put,
    path = "/api/resources/{id}",
    request_body = StaticResourceInput,
    responses(
        (status = 200, description = "Resource updated", body = ApiResponseResource),
        (status = 404, description = "Not found", body = ApiResponseEmpty),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
async fn update_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match manager.update_resource(id, input).await {
        Ok(Some(resource)) => (StatusCode::OK, Json(ApiResponse::success(resource))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/resources/{id}",
    responses(
        (status = 200, description = "Resource deleted", body = ApiResponseEmpty),
        (status = 404, description = "Not found", body = ApiResponseEmpty),
        (status = 400, description = "Database error", body = ApiResponseEmpty)
    ),
    params(
        ("id" = i32, Path, description = "Resource id")
    )
)]
async fn delete_resource(
    State(manager): State<Arc<StaticResourceManager>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    match manager.delete_resource(id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponseEmpty::success())).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponseEmpty::error("Resource not found".to_string())),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponseEmpty::error(format!("Database error: {}", e))),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_resources,
        get_resource,
        create_resource,
        update_resource,
        delete_resource
    ),
    components(
        schemas(
            StaticResource,
            StaticResourceInput,
            ApiResponseResource,
            ApiResponseResourceList,
            ApiResponseEmpty
        )
    ),
    tags(
        (name = "Resources", description = "Static resources CRUD")
    )
)]
struct ApiDoc;

static OPENAPI: Lazy<utoipa::openapi::OpenApi> = Lazy::new(|| ApiDoc::openapi());

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(OPENAPI.clone())
}

async fn swagger_ui() -> Response {
    const HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
  <title>LiRAYS-SCADA API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      SwaggerUIBundle({
        url: '/api-docs/openapi.json',
        dom_id: '#swagger-ui'
      });
    };
  </script>
</body>
</html>"#;
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(HTML))
        .unwrap()
}

pub async fn run_http_server(host: &str, port: u16, db_file: &str, tls_config: Option<ServerTlsConfig>) {
    // Initialize the static resource manager
    let resource_manager = Arc::new(
        StaticResourceManager::new(db_file)
            .await
            .expect("Failed to initialize static resource manager")
    );
    resource_manager.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        // Static resource routes
        .route("/api/resources", get(get_all_resources).post(create_resource))
        .route("/api/resources/{id}", get(get_resource).put(update_resource).delete(delete_resource))
        // Docs
        .route("/api-docs/openapi.json", get(openapi_spec))
        .route("/swagger", get(swagger_ui))
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
