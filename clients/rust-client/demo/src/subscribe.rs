use anyhow::Result;
use futures_util::StreamExt;
use lirays_ws_client::{Client, FloatVar};
use tokio::time::{sleep, Duration};

/// Simple subscription demo: two subscribers listen to a handful of vars while a writer updates them.
pub async fn run(host: &str, port: i64, tls: bool) -> Result<()> {
    // Setup client and create a small namespace
    let client = Client::connect(host, port, tls).await?;
    let root = format!("/sub_demo_{}", chrono::Utc::now().timestamp());
    client.create_folders(vec![root.trim_start_matches('/').to_string()], None, 5_000).await?;

    let vars: Vec<FloatVar> = (0..5)
        .map(|i| FloatVar { name: format!("v{i}"), unit: Some("unit".into()), min: None, max: None })
        .collect();
    client.create_float_variables(vars.clone(), Some(root.clone()), 5_000).await?;

    let var_ids: Vec<String> = vars.iter().map(|v| format!("{}/{}", root, v.name)).collect();

    // Clone for tasks
    let writer_ids = var_ids.clone();
    let sub_ids_a = var_ids.clone();
    let sub_ids_b = var_ids.clone();

    // Subscriber A: prints every event
    let host_a = host.to_string();
    let client_a = Client::connect(&host_a, port, tls).await?;
    let handle_a = tokio::spawn(async move {
        let mut stream = client_a.subscribe_var_values(sub_ids_a, 5_000).await.expect("sub a");
        while let Some((id, val)) = stream.next().await {
            println!("[sub A] {} => {:?}", id, val);
        }
    });

    // Subscriber B: filters only even-indexed vars and prints count
    let host_b = host.to_string();
    let client_b = Client::connect(&host_b, port, tls).await?;
    let handle_b = tokio::spawn(async move {
        let mut stream = client_b.subscribe_var_values(sub_ids_b, 5_000).await.expect("sub b");
        let mut count = 0u64;
        while let Some((id, val)) = stream.next().await {
            if id.ends_with('0') || id.ends_with('2') || id.ends_with('4') {
                count += 1;
                println!("[sub B] count={} last={} -> {:?}", count, id, val);
            }
        }
    });

    // Writer: updates all vars every 500ms for 10 seconds
    let host_w = host.to_string();
    let writer = tokio::spawn(async move {
        let writer_client = Client::connect(&host_w, port, tls).await.expect("writer connect");
        let mut n = 0f64;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while tokio::time::Instant::now() < deadline {
            n += 1.0;
            let vals: Vec<f64> = (0..writer_ids.len()).map(|i| n + i as f64).collect();
            let _ = writer_client.set_float_variables(writer_ids.clone(), vals, 5_000).await;
            sleep(Duration::from_millis(500)).await;
        }
    });

    writer.await?;
    // Give subscribers a moment to drain
    sleep(Duration::from_secs(1)).await;
    handle_a.abort();
    handle_b.abort();
    Ok(())
}
