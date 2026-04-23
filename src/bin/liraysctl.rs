use std::process;

#[tokio::main]
async fn main() {
    if let Err(err) = rustmod::liraysctl::run().await {
        eprintln!("{err}");
        process::exit(1);
    }
}
