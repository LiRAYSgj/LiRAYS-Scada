mod rtdata;

use log::info;
use std::{{env, path::Path}, fs::create_dir_all};
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
    let d_dir_str = env::var("DATA_DIR").unwrap_or(default_data_dir);
    let data_dir = Path::new(&d_dir_str);

    let rt_db_dir = data_dir.join("rt_data");
    let static_db_file = data_dir.join("static.db");
    create_dir_all(&rt_db_dir).unwrap();

    info!("Starting server on {host}:{port}");
    let server_handle = task::spawn(async move {
        run_server(&host, port, &rt_db_dir.to_str().unwrap()).await;
    });

    info!("Starting HTTP server on {http_host}:{http_port}");
    let http_handle = task::spawn(async move {
        run_http_server(&http_host, http_port, &static_db_file.to_str().unwrap()).await;
    });

    // Wait for both servers to finish
    let _ = tokio::try_join!(server_handle, http_handle);
}
