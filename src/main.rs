mod http;
mod rtdata;
mod tls;
mod migration;

use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use log::info;
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::task;

use http::run_http_server;
use tls::ServerTlsConfig;


fn generate_self_signed_cert(out_dir: &Path) -> (PathBuf, PathBuf) {
    fs::create_dir_all(out_dir).unwrap();

    // Simple self-signed certificate for local usage
    let cert: CertifiedKey = generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    let cert_path = out_dir.join("server.crt");
    let key_path = out_dir.join("server.key");

    fs::write(&cert_path, cert_pem).unwrap();
    fs::write(&key_path, key_pem).unwrap();

    (cert_path, key_path)
}

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    let cwd = env::current_dir().unwrap();
    let default_data_dir = cwd.join("data_dir").to_str().unwrap().to_string();

    // Get environment variables or use defaults
    let host = env::var("BIND_HOST")
        .unwrap_or("0.0.0.0".to_string());
    let port = env::var("BIND_PORT")
        .unwrap_or("8245".to_string())
        .parse::<u16>()
        .unwrap();
    let d_dir_str = env::var("DATA_DIR").unwrap_or(default_data_dir);
    let data_dir = Path::new(&d_dir_str);
    let tls_enabled = env::var("WS_TLS_ENABLE")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    let tls_cert_path = env::var("WS_TLS_CERT_PATH").ok();
    let tls_key_path = env::var("WS_TLS_KEY_PATH").ok();

    let (server_tls, ws_schema) = if tls_enabled {
        let (cert_path, key_path) = match (tls_cert_path, tls_key_path) {
            (Some(cert_path), Some(key_path)) => (PathBuf::from(cert_path), PathBuf::from(key_path)),
            _ => generate_self_signed_cert(&data_dir.join("certificates"))
        };
        (Some(ServerTlsConfig::new(cert_path, key_path)), "wss")
    } else {
        (None, "ws")
    };

    let rt_db_dir = data_dir.join("rt_data");
    let static_db_file = data_dir.join("static.db");
    fs::create_dir_all(&rt_db_dir).unwrap();

    let http_tls = server_tls.clone();

    let http_schema = if http_tls.is_some() { "https" } else { "http" };
    info!("Starting unified server on {http_schema}/{ws_schema}://{host}:{port}");
    let http_handle = task::spawn(async move {
        run_http_server(
            &host,
            port,
            &rt_db_dir.to_str().unwrap(),
            &static_db_file.to_str().unwrap(),
            http_tls,
        )
        .await;
    });

    let _ = http_handle.await;
}
