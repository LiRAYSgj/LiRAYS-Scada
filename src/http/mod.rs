mod resources;

use std::{
    io,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::serve::Listener;
use axum::{
    Router,
    extract::State,
    http::{StatusCode, Uri, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use constant_time_eq::constant_time_eq;
use hmac::{Hmac, Mac};
use log::{error, info};
use rand::{RngCore, rngs::OsRng};
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::Duration as CookieDuration;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use utoipa::ToSchema;

use super::tls::{ServerTlsConfig, build_tls_acceptor};
use crate::rtdata::{metrics::Metrics, variable::VariableManager};
use resources::user::service::UserManager;
use resources::views::service::{View, ViewManager};

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiResponseView {
    success: bool,
    data: Option<View>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiResponseViewList {
    success: bool,
    data: Option<Vec<View>>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiResponseViewPage {
    success: bool,
    data: Option<resources::views::service::ViewPage>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiResponseEmpty {
    success: bool,
    data: Option<()>,
    message: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub(super) struct ApiTokenResponse {
    token: String,
    refresh_token: String,
    expires_at: i64,
    refresh_expires_at: i64,
}

impl ApiResponseEmpty {
    pub(super) fn success() -> Self {
        Self {
            success: true,
            data: Some(()),
            message: None,
        }
    }

    pub(super) fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

impl<T> ApiResponse<T> {
    pub(super) fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }
    }
}

#[derive(Clone)]
pub(super) struct AuthConfig {
    enabled: bool,
    secret: Arc<Vec<u8>>,
    access_ttl: Duration,
    refresh_ttl: Duration,
    secure_cookies: bool,
}

#[derive(Clone)]
pub(super) struct AppState {
    views: Arc<ViewManager>,
    users: Arc<UserManager>,
    auth: AuthConfig,
    var_manager: Arc<VariableManager>,
}

fn hmac_sign(secret: &[u8], data: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC init");
    mac.update(data);
    let sig = mac.finalize().into_bytes();
    URL_SAFE_NO_PAD.encode(sig)
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub(super) enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
}

#[derive(Serialize, Deserialize)]
pub(super) struct SessionClaims {
    user: i32,
    exp: i64,
    #[serde(rename = "typ")]
    token_type: TokenType,
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

pub(super) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64
}

pub(super) fn token_is_valid(auth: &AuthConfig, token: &str, expected: TokenType) -> bool {
    decode_session(&auth.secret, token)
        .map(|claims| claims.token_type == expected && claims.exp > now_ts())
        .unwrap_or(false)
}

fn bearer_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer ").map(|s| s.to_string()))
}

fn token_from_query(uri: &Uri) -> Option<String> {
    uri.query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let val = parts.next().unwrap_or("");
            if key == "token" {
                Some(val.to_string())
            } else {
                None
            }
        })
    })
}

pub(super) fn issue_token(
    auth: &AuthConfig,
    user: i32,
    token_type: TokenType,
    ttl: Duration,
) -> Result<(String, i64), StatusCode> {
    let exp = now_ts() + ttl.as_secs() as i64;
    let claims = SessionClaims {
        user,
        exp,
        token_type,
    };
    let token =
        encode_session(&auth.secret, &claims).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((token, exp))
}

pub(super) fn session_cookie(token: String, secure: bool, ttl: Duration) -> Cookie<'static> {
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

pub(super) fn refresh_cookie(token: String, secure: bool, ttl: Duration) -> Cookie<'static> {
    let mut cookie = Cookie::build(("lirays_refresh", token))
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

async fn run_migrations_safe(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), sea_orm_migration::DbErr> {
    match crate::migration::Migrator::up(db, None).await {
        Ok(_) => Ok(()),
        Err(sea_orm_migration::DbErr::Exec(err)) => {
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
    if path.starts_with("/auth/")
        || path.starts_with("/_app/")
        || path == "/robots.txt"
        || path.starts_with("/.well-known/")
        || path.ends_with(".ico")
    {
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

    let token = bearer_from_headers(req.headers())
        .or_else(|| token_from_query(req.uri()))
        .or_else(|| jar.get("lirays_session").map(|c| c.value().to_string()));

    if let Some(token) = token {
        if token_is_valid(&state.auth, &token, TokenType::Access) {
            return Ok(next.run(req).await);
        }
    }

    if path.starts_with("/api") || path.starts_with("/ws") {
        Err(StatusCode::UNAUTHORIZED)
    } else {
        Ok(Redirect::temporary("/auth/login").into_response())
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

pub async fn run_http_server(
    host: &str,
    port: u16,
    rt_db_dir: &str,
    static_db_file: &str,
    tls_config: Option<ServerTlsConfig>,
) {
    let db_url = format!("sqlite://{}?mode=rwc", static_db_file);
    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect database");
    run_migrations_safe(&db)
        .await
        .expect("Failed to run migrations");

    let metrics = Arc::new(Metrics::new_from_env());
    if metrics.enabled() {
        Metrics::spawn_logger(metrics.clone());
    }
    let var_manager = Arc::new(VariableManager::new(rt_db_dir, metrics));

    let flush_ms: u64 = std::env::var("PERSIST_FLUSH_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15_000);
    let _flush_handle = var_manager.clone().start_flush_loop(flush_ms);

    let view_manager = Arc::new(ViewManager::new(db.clone()));
    view_manager
        .ensure_initialized()
        .await
        .expect("Failed to initialize views storage");
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
        access_ttl: Duration::from_secs(60 * 60),       // 1h
        refresh_ttl: Duration::from_secs(60 * 60 * 24), // 24h
        secure_cookies: tls_config.is_some(),
    };

    let app_state = Arc::new(AppState {
        views: view_manager,
        users: user_manager,
        auth: auth_config,
        var_manager: var_manager.clone(),
    });

    let app = Router::new()
        .route(
            "/api/views",
            get(resources::get_all_views).post(resources::create_view),
        )
        .route(
            "/api/views/entry-point",
            get(resources::get_entry_point_view),
        )
        .route(
            "/api/views/{id}",
            get(resources::get_view)
                .put(resources::update_view)
                .delete(resources::delete_view),
        )
        .route(
            "/api/views/{id}/entry-point",
            axum::routing::put(resources::set_entry_point_view),
        )
        .route("/ws", get(resources::ws_handler))
        .route("/api-docs/openapi.json", get(resources::openapi_spec))
        .route("/swagger", get(resources::swagger_ui))
        .route(
            "/auth/setup",
            get(resources::setup_get).post(resources::setup_post),
        )
        .route(
            "/auth/login",
            get(resources::login_get).post(resources::login_post),
        )
        .route("/auth/status", get(resources::auth_status))
        .route("/auth/token", post(resources::login_api))
        .route("/auth/refresh", post(resources::refresh_api))
        .route("/auth/logout", get(resources::logout))
        .fallback(resources::serve_static)
        .with_state(app_state.clone())
        .layer(middleware::from_fn_with_state(app_state, auth_middleware));

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host/port");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    let shutdown_vm = var_manager.clone();
    async fn shutdown_signal(vm: Arc<VariableManager>) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
            tokio::select! {
                _ = sigterm.recv() => {},
                _ = sigint.recv() => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        info!("Shutdown signal received; flushing dirty cache");
        vm.flush_dirty_now().await;
        info!("Cache flush complete; exiting");
    }

    if let Some(cfg) = tls_config {
        let acceptor = build_tls_acceptor(&cfg).expect("Failed to build TLS acceptor for HTTP");
        let incoming = TlsIncoming::new(listener, acceptor);
        axum::serve(incoming, app)
            .with_graceful_shutdown(shutdown_signal(shutdown_vm))
            .await
            .expect("HTTPS server error");
    } else {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown_vm))
            .await
            .expect("HTTP server error");
    }
}
