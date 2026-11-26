use std::process;

#[tokio::main]
async fn main() {
    if let Err(err) = scada::liraysctl::run().await {
        eprintln!("{err}");
        process::exit(1);
    }
}
