use anyhow::Result;
use env_logger::Env;
use log::info;

mod extress_test;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let demo = args.get(0).cloned().unwrap_or_else(|| "extress_test".into());
    let do_setup = args.iter().any(|a| a == "--setup");
    info!("demo seleccionado: {demo}");

    match demo.as_str() {
        "extress_test" | "stress" => {
            if do_setup {
                info!("ejecutando setup de 1M variables float...");
                extress_test::setup_namespace("127.0.0.1", 8245, false).await?;
            }
            extress_test::run("127.0.0.1", 8245, false).await?
        }
        other => {
            anyhow::bail!("demo desconocido: {other}");
        }
    }

    // No se llega aquí en el loop infinito del stress test, pero dejamos el cierre por si se agregan otros demos.
    // client.disconnect().await?;
    Ok(())
}
