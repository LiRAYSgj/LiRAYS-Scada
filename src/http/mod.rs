pub mod resources;

mod response;
mod state;

use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::Router;
use axum::serve::Listener;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use log::{error, info};
use rand::{RngCore, rngs::OsRng};
use resources::{
    auth::{self, session::SessionManager},
    docs, patoken, static_files, user, views, ws,
};
pub use response::{ApiResponse, ApiResponseEmpty, ApiResponseView, ApiResponseViewPage};
use sea_orm::{Database, DatabaseConnection};
use state::AuthConfig;
pub use state::{AppState, AuthContext};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

use super::tls::{ServerTlsConfig, build_tls_acceptor};
use crate::rtdata::{metrics::Metrics, variable::VariableManager};

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
    rt_db_dir: &Path,
    sessions_dir: &Path,
    static_db_file: &Path,
    tls_config: Option<ServerTlsConfig>,
    metrics_dir: PathBuf,
    metrics_real_time: bool,
    metrics_historic: bool,
    flush_ms: u64,
    auth_enabled: bool,
    auth_access_ttl: u64,
    auth_refresh_ttl: u64,
    auth_secret: Option<Vec<u8>>,
) -> Result<(), String> {
    let db_url = format!("sqlite://{}?mode=rwc", static_db_file.to_string_lossy());
    let db = match Database::connect(db_url).await {
        Ok(db) => db,
        Err(err) => return Err(format!("Failed to connect database: {err}")),
    };
    if let Err(err) = ensure_schema(&db).await {
        return Err(err);
    }

    let metrics = Arc::new(Metrics::new(
        metrics_dir,
        metrics_real_time,
        metrics_historic,
    ));
    if metrics.enabled() {
        Metrics::spawn_logger(metrics.clone());
    }
    let rt_db_dir_str = rt_db_dir
        .to_str()
        .ok_or_else(|| format!("Invalid UTF-8 in rt_db_dir path: {}", rt_db_dir.display()))?;
    let var_manager = Arc::new(
        VariableManager::new(rt_db_dir_str, metrics)
            .map_err(|e| format!("Failed to initialize variable manager: {e}"))?,
    );

    let _flush_handle = var_manager.clone().start_flush_loop(flush_ms);

    let view_manager = Arc::new(views::service::ViewManager::new(db.clone()));
    let user_manager = Arc::new(user::service::UserManager::new(db.clone()));
    let patoken_manager = Arc::new(patoken::service::PATokenManager::new(db.clone()));

    let secret = match auth_secret {
        Some(sec_) => sec_,
        None => {
            if auth_enabled {
                return Err("AUTH_ENABLED=true but no auth.secret provided; set AUTH_SECRET or auth.secret in settings.".to_string());
            } else {
                let mut buf = vec![0u8; 32];
                OsRng.fill_bytes(&mut buf);
                buf
            }
        }
    };
    let auth_config = AuthConfig {
        enabled: auth_enabled,
        secret: Arc::new(secret),
        access_ttl: Duration::from_secs(auth_access_ttl),
        refresh_ttl: Duration::from_secs(auth_refresh_ttl),
        secure_cookies: tls_config.is_some(),
    };

    let sessions_dir_str = sessions_dir.to_str().ok_or_else(|| {
        format!(
            "Invalid UTF-8 in sessions_dir path: {}",
            sessions_dir.display()
        )
    })?;
    let session_secret = URL_SAFE_NO_PAD.encode(auth_config.secret.as_slice());
    let session_manager = Arc::new(
        SessionManager::new(
            sessions_dir_str,
            &session_secret,
            &session_secret,
            auth_config.access_ttl.as_secs(),
            auth_config.refresh_ttl.as_secs(),
        )
        .map_err(|e| format!("Failed to initialize session manager: {e}"))?,
    );
    let _session_purge_handle = session_manager.clone().start_purge_loop();

    let app_state = Arc::new(AppState {
        views: view_manager,
        users: user_manager,
        patoken_manager,
        auth: auth_config,
        var_manager: var_manager.clone(),
        session_manager: session_manager.clone(),
    });

    let app = Router::new();
    let app = ws::mount(app, app_state.clone());
    let app = views::mount(app, app_state.clone());
    let app = auth::mount(app);
    let app = docs::mount(app);
    let app = static_files::with_fallback(app).with_state(app_state.clone());

    let addr: std::net::SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid host/port '{host}:{port}': {e}"))?;

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(err) => return Err(format!("Failed to bind {addr}: {err}")),
    };

    let shutdown_vm = var_manager.clone();
    async fn shutdown_signal(vm: Arc<VariableManager>) {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            match (
                signal(SignalKind::terminate()),
                signal(SignalKind::interrupt()),
            ) {
                (Ok(mut sigterm), Ok(mut sigint)) => {
                    tokio::select! {
                        _ = sigterm.recv() => {},
                        _ = sigint.recv() => {},
                    }
                }
                (Err(e), _) | (_, Err(e)) => {
                    error!("Failed to install UNIX signal handlers: {e}; falling back to ctrl_c");
                    let _ = tokio::signal::ctrl_c().await;
                }
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
        let acceptor = build_tls_acceptor(&cfg)
            .map_err(|e| format!("Failed to build TLS acceptor for HTTP: {e}"))?;
        let incoming = TlsIncoming::new(listener, acceptor);
        axum::serve(incoming, app)
            .with_graceful_shutdown(shutdown_signal(shutdown_vm))
            .await
            .map_err(|e| format!("HTTPS server error: {e}"))?;
    } else {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(shutdown_vm))
            .await
            .map_err(|e| format!("HTTP server error: {e}"))?;
    }

    Ok(())
}

async fn ensure_schema(db: &DatabaseConnection) -> Result<(), String> {
    user::ensure_schema(db)
        .await
        .map_err(|e| format!("Failed to ensure users schema: {e}"))?;
    patoken::ensure_schema(db)
        .await
        .map_err(|e| format!("Failed to ensure patokens schema: {e}"))?;
    views::ensure_schema(db)
        .await
        .map_err(|e| format!("Failed to ensure views schema: {e}"))?;
    Ok(())
}
