use anyhow::Result;
use env_logger::Env;

mod basic;
mod tree_stress;
mod data_stress;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let demo = args.get(0).cloned().unwrap_or_else(|| "basic".into());

    match demo.as_str() {
        "basic" => {
            basic::run("127.0.0.1", 8245, false).await;
            Ok(())
        }
        "tree_stress" | "tree" => tree_stress::run("127.0.0.1", 8245, false).await,
        "data_stress" | "data" => data_stress::run("127.0.0.1", 8245, false).await,
        other => anyhow::bail!("unknown demo: {other}"),
    }
}
