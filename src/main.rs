mod rtdata;

use log::info;
use std::env;
use tokio::task;
use rtdata::{server::run_server, http::run_http_server};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    let cwd = env::current_dir().unwrap();
    let default_data_dir = cwd.join("data_dir").to_str().unwrap().to_string();

    // Get environment variables or use defaults
    let host = env::var("BIND_HOST").unwrap_or("0.0.0.0".to_string());
    let port = env::var("BIND_SERVER_PORT").unwrap_or("8245".to_string()).parse::<u16>().unwrap();
    let http_host = env::var("BIND_HTTP_HOST").unwrap_or("0.0.0.0".to_string());
    let http_port = env::var("BIND_HTTP_PORT").unwrap_or("8246".to_string()).parse::<u16>().unwrap();
    let db_dir = env::var("DATA_DIR").unwrap_or(default_data_dir);

    info!("Starting server on {host}:{port} with db_dir: {db_dir}");
    let server_handle = task::spawn(async move {
        run_server(&host, port, &db_dir).await;
    });

    info!("Starting HTTP server on {http_host}:{http_port}");
    let http_handle = task::spawn(async move {
        run_http_server(&http_host, http_port).await;
    });

    // Wait for both servers to finish
    let _ = tokio::try_join!(server_handle, http_handle);
}
