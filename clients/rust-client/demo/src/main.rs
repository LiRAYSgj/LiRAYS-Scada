use anyhow::Result;
use env_logger::Env;
use lirays_ws_client::Client;
use log::info;

mod extress_test;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let demo = std::env::args().nth(1).unwrap_or_else(|| "extress_test".into());
    info!("demo seleccionado: {demo}");

    let client = Client::connect("127.0.0.1", 8245, false).await?;
    info!("connected");

    match demo.as_str() {
        "extress_test" | "stress" => extress_test::run(&client).await?,
        other => {
            anyhow::bail!("demo desconocido: {other}");
        }
    }

    // No se llega aquí en el loop infinito del stress test, pero dejamos el cierre por si se agregan otros demos.
    // client.disconnect().await?;
    Ok(())
}
