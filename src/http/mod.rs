mod model;

use std::{
    io,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use axum::serve::Listener;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use constant_time_eq::constant_time_eq;
use hmac::{Hmac, Mac};
use include_dir::{include_dir, Dir};
use log::error;
use model::resource::service::{
    StaticResource, StaticResourceInput, StaticResourceManager,
};
use model::user::service::{UserCredentials, UserManager};
use once_cell::sync::Lazy;
use rand::{rngs::OsRng, RngCore};
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::Duration as CookieDuration;
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

#[derive(Clone)]
struct AuthConfig {
    enabled: bool,
    secret: Arc<Vec<u8>>,
    ttl: Duration,
    secure_cookies: bool,
}

#[derive(Clone)]
struct AppState {
    resources: Arc<StaticResourceManager>,
    users: Arc<UserManager>,
    auth: AuthConfig,
}

fn hmac_sign(secret: &[u8], data: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC init");
    mac.update(data);
    let sig = mac.finalize().into_bytes();
    URL_SAFE_NO_PAD.encode(sig)
}

#[derive(Serialize, Deserialize)]
struct SessionClaims {
    user: i32,
    exp: i64,
}

fn encode_session(secret: &[u8], claims: &SessionClaims) -> Result<String, serde_json::Error> {
    let payload = serde_json::to_vec(claims)?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload);
    let sig = hmac_sign(secret, payload_b64.as_bytes());
    Ok(format!("{}.{}", payload_b64, sig))
}

fn decode_session(secret: &[u8], token: &str) -> Option<SessionClaims> {
    let mut parts = token.split('.');
    let payload_b64 = parts.next()?;
    let sig = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let expected = hmac_sign(secret, payload_b64.as_bytes());
    if !constant_time_eq(expected.as_bytes(), sig.as_bytes()) {
        return None;
    }
    let payload = URL_SAFE_NO_PAD.decode(payload_b64.as_bytes()).ok()?;
    serde_json::from_slice(&payload).ok()
}

async fn run_migrations_safe(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm_migration::DbErr> {
    match crate::migration::Migrator::up(db, None).await {
        Ok(_) => Ok(()),
        Err(sea_orm_migration::DbErr::Exec(err)) => {
            // If already applied, skip
            if err.to_string().contains("seaql_migrations.version") {
                Ok(())
            } else {
                Err(sea_orm_migration::DbErr::Exec(err))
            }
        }
        Err(e) => Err(e),
    }
}

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.auth.enabled {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    if path.starts_with("/auth/") {
        return Ok(next.run(req).await);
    }

    // If no admin set, force setup
    let admin_exists = state
        .users
        .admin_exists()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !admin_exists {
        return Ok(Redirect::temporary("/auth/setup").into_response());
    }

    if let Some(cookie) = jar.get("lirays_session") {
        if let Some(claims) = decode_session(&state.auth.secret, cookie.value()) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as i64;
            if claims.exp > now {
                return Ok(next.run(req).await);
            }
        }
    }

    Ok(Redirect::temporary("/auth/login").into_response())
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
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    match state.resources.get_resource(id).await {
        Ok(Some(resource)) => {
            (StatusCode::OK, Json(ApiResponse::success(resource))).into_response()
        }
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
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.resources.get_all_resources().await {
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
    State(state): State<Arc<AppState>>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match state.resources.create_resource(input).await {
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
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
    Json(input): Json<StaticResourceInput>,
) -> impl IntoResponse {
    match state.resources.update_resource(id, input).await {
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
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> impl IntoResponse {
    match state.resources.delete_resource(id).await {
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

fn session_cookie(token: String, secure: bool, ttl: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::build(("lirays_session", token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(ttl.as_secs() as i64))
        .build();
    if secure {
        cookie.set_secure(true);
    }
    cookie
}

async fn setup_get() -> impl IntoResponse {
    const FORM: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Set admin password</title>
  <style>
    :root { color-scheme: light dark; }
    body {
      margin:0; padding:0;
      font-family: -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
      background: radial-gradient(circle at 20% 20%, #1e3a8a22, transparent 35%),
                  radial-gradient(circle at 80% 0%, #0ea5e922, transparent 40%),
                  #0b1220;
      color: #e5e7eb;
      display:flex; align-items:center; justify-content:center; min-height:100vh;
    }
    .card {
      background: #0f172a;
      border: 1px solid #1f2937;
      padding: 28px;
      border-radius: 14px;
      width: min(420px, 90vw);
      box-shadow: 0 20px 50px rgba(0,0,0,0.45);
    }
    h1 { margin: 0 0 8px; font-size: 24px; }
    p { margin: 0 0 18px; color: #9ca3af; line-height: 1.5; }
    label { display:block; margin:14px 0 6px; font-weight:600; }
    input {
      width: 100%; padding: 12px 14px;
      border-radius: 10px; border: 1px solid #1f2937;
      background: #111827; color: #e5e7eb;
      font-size: 15px;
    }
    button {
      margin-top: 18px; width: 100%;
      padding: 12px 16px;
      border: none; border-radius: 10px;
      background: linear-gradient(135deg, #2563eb, #0ea5e9);
      color: white; font-weight: 700; font-size: 15px;
      cursor: pointer; transition: transform 120ms ease, filter 120ms ease;
    }
    button:hover { transform: translateY(-1px); filter: brightness(1.05); }
    .note { font-size: 13px; color: #9ca3af; margin-top: 10px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Set admin password</h1>
    <p>First-time setup: define the password for user <strong>admin</strong>.</p>
    <form method="post" action="/auth/setup">
      <label for="password">New password</label>
      <input id="password" name="password" type="password" required minlength="6" autofocus />
      <button type="submit">Save and continue</button>
      <div class="note">Stored server-side as an Argon2 hash.</div>
    </form>
  </div>
</body>
</html>"#;
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(FORM))
        .unwrap()
}

#[derive(Deserialize)]
struct PasswordForm {
    password: String,
}

async fn setup_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::extract::Form(form): axum::extract::Form<PasswordForm>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }
    if state.users.admin_exists().await.unwrap_or(true) {
        return Redirect::to("/auth/login").into_response();
    }
    if form.password.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Password required").into_response();
    }
    if let Err(e) = state.users.create_admin(form.password).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error creando admin: {e}"),
        )
            .into_response();
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64;
    let claims = SessionClaims {
        user: 1,
        exp: now + state.auth.ttl.as_secs() as i64,
    };
    let token = encode_session(&state.auth.secret, &claims)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();
    let cookie = session_cookie(token, state.auth.secure_cookies, state.auth.ttl);
    let jar = jar.add(cookie);
    (jar, Redirect::to("/")).into_response()
}

async fn login_get() -> impl IntoResponse {
    const FORM: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Login</title>
  <style>
    :root { color-scheme: light dark; }
    body {
      margin:0; padding:0;
      font-family: -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
      background: radial-gradient(circle at 10% 10%, #16a34a22, transparent 35%),
                  radial-gradient(circle at 80% 20%, #2563eb22, transparent 40%),
                  #0b1220;
      color: #e5e7eb;
      display:flex; align-items:center; justify-content:center; min-height:100vh;
    }
    .card {
      background: #0f172a;
      border: 1px solid #1f2937;
      padding: 28px;
      border-radius: 14px;
      width: min(420px, 90vw);
      box-shadow: 0 20px 50px rgba(0,0,0,0.45);
    }
    h1 { margin: 0 0 8px; font-size: 24px; }
    p { margin: 0 0 18px; color: #9ca3af; line-height: 1.5; }
    label { display:block; margin:14px 0 6px; font-weight:600; }
    input {
      width: 100%; padding: 12px 14px;
      border-radius: 10px; border: 1px solid #1f2937;
      background: #111827; color: #e5e7eb;
      font-size: 15px;
    }
    button {
      margin-top: 18px; width: 100%;
      padding: 12px 16px;
      border: none; border-radius: 10px;
      background: linear-gradient(135deg, #16a34a, #22c55e);
      color: white; font-weight: 700; font-size: 15px;
      cursor: pointer; transition: transform 120ms ease, filter 120ms ease;
    }
    button:hover { transform: translateY(-1px); filter: brightness(1.05); }
    .hint { font-size: 13px; color: #9ca3af; margin-top: 10px; }
  </style>
</head>
<body>
  <div class="card">
    <h1>Sign in</h1>
    <p>Use the <strong>admin</strong> account.</p>
    <form method="post" action="/auth/login">
      <label for="username">Username</label>
      <input id="username" name="username" value="admin" required />
      <label for="password">Password</label>
      <input id="password" name="password" type="password" required />
      <button type="submit">Sign in</button>
      <div class="hint">If admin doesn’t exist yet, go to /auth/setup first.</div>
    </form>
  </div>
</body>
</html>"#;
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(FORM))
        .unwrap()
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_post(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::extract::Form(form): axum::extract::Form<LoginForm>,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }
    let creds = UserCredentials {
        username: form.username,
        password: form.password,
    };
    match state.users.verify(&creds).await {
        Ok(true) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs() as i64;
            let claims = SessionClaims {
                user: 1,
                exp: now + state.auth.ttl.as_secs() as i64,
            };
            let token = encode_session(&state.auth.secret, &claims)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                .unwrap();
            let cookie = session_cookie(token, state.auth.secure_cookies, state.auth.ttl);
            let jar = jar.add(cookie);
            (jar, Redirect::to("/")).into_response()
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            "Credenciales inválidas",
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error de base de datos: {e}"),
        )
            .into_response(),
    }
}

async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> impl IntoResponse {
    if !state.auth.enabled {
        return Redirect::to("/").into_response();
    }
    let mut cookie = Cookie::build(("lirays_session", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(0))
        .build();
    if state.auth.secure_cookies {
        cookie.set_secure(true);
    }
    let jar = jar.add(cookie);
    (jar, Redirect::to("/auth/login")).into_response()
}

pub async fn run_http_server(host: &str, port: u16, db_file: &str, tls_config: Option<ServerTlsConfig>) {
    // rwc ensures the sqlite file is created if missing
    let db_url = format!("sqlite://{}?mode=rwc", db_file);
    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect database");
    // Run migrations once for all models
    run_migrations_safe(&db)
        .await
        .expect("Failed to run migrations");

    let resource_manager = Arc::new(StaticResourceManager::new(db.clone()));
    let user_manager = Arc::new(UserManager::new(db.clone()));

    let auth_enabled = std::env::var("AUTH_ENABLED")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    let secret = std::env::var("AUTH_SECRET")
        .ok()
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| {
            let mut buf = vec![0u8; 32];
            OsRng.fill_bytes(&mut buf);
            buf
        });
    let auth_config = AuthConfig {
        enabled: auth_enabled,
        secret: Arc::new(secret),
        ttl: Duration::from_secs(60 * 60 * 24), // 24h
        secure_cookies: tls_config.is_some(),
    };

    let app_state = Arc::new(AppState {
        resources: resource_manager,
        users: user_manager,
        auth: auth_config,
    });

    let app = Router::new()
        // Static resource routes
        .route("/api/resources", get(get_all_resources).post(create_resource))
        .route("/api/resources/{id}", get(get_resource).put(update_resource).delete(delete_resource))
        // Docs
        .route("/api-docs/openapi.json", get(openapi_spec))
        .route("/swagger", get(swagger_ui))
        // Auth routes
        .route("/auth/setup", get(setup_get).post(setup_post))
        .route("/auth/login", get(login_get).post(login_post))
        .route("/auth/logout", get(logout))
        // Fallback to static file serving
        .fallback(serve_static)
        // Add state to the app
        .with_state(app_state.clone())
        // Auth middleware on every route/fallback
        .layer(middleware::from_fn_with_state(
            app_state,
            auth_middleware,
        ));

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
