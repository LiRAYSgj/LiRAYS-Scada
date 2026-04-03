use anyhow::Result;
use env_logger::Env;

mod basic;
mod tree_stress;
mod data_stress;
mod bulk_test;
mod subscribe;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let demo = args.get(0).cloned().unwrap_or_else(|| "basic".into());
    let port = 8245;

    match demo.as_str() {
        "basic" => {
            basic::run("127.0.0.1", port, false).await;
            Ok(())
        }
        "tree_stress" | "tree" => tree_stress::run("127.0.0.1", port, false).await,
        "data_stress" | "data" => data_stress::run("127.0.0.1", port, false).await,
        "bulk_test" | "bulk" => bulk_test::run("127.0.0.1", port, false).await,
        "subscribe" | "sub" => subscribe::run("127.0.0.1", port, false).await,
        other => anyhow::bail!("unknown demo: {other}"),
    }
}
