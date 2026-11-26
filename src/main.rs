#[tokio::main]
async fn main() {
    scada::app::run().await;
}
