use anyhow::Result;
use rand::{seq::SliceRandom, Rng};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use lirays_ws_client::{Client, FloatVar};

const NUM_FOLDERS: usize = 200;
const VARS_PER_FOLDER: usize = 5000;
const NUM_CLIENTS: usize = 30;
const PCT_TOUCH: usize = 10; // percentage of vars each client touches per tick
const RUN_SECS: u64 = 60;
const SET_CLIENT_RATIO: f64 = 0.10; // 10% of clients perform sets; rest only gets

pub async fn run(host: &str, port: i64, tls: bool) -> Result<()> {
    // Build namespace once
    let setup_client = Client::connect(host, port, tls).await?;
    let root_name = format!("DataStress_{}", chrono::Utc::now().timestamp_millis());
    let root_path = format!("/{}", root_name);
    setup_client
        .create_folders(vec![root_name.clone()], None, 10_000)
        .await
        .expect("root create");

    let mut all_var_ids = Vec::with_capacity(NUM_FOLDERS * VARS_PER_FOLDER);
    for fi in 0..NUM_FOLDERS {
        let folder_name = format!("f{fi}");
        let folder_path = format!("{}/{}", root_path, folder_name);
        setup_client
            .create_folders(vec![folder_name.clone()], Some(root_path.clone()), 10_000)
            .await
            .expect("folder create");

        let vars: Vec<FloatVar> = (0..VARS_PER_FOLDER)
            .map(|vi| FloatVar { name: format!("v{vi}"), unit: None, min: None, max: None })
            .collect();
        setup_client
            .create_float_variables(vars, Some(folder_path.clone()), 10_000)
            .await
            .expect("vars create");

        for vi in 0..VARS_PER_FOLDER {
            all_var_ids.push(format!("{}/v{}", folder_path, vi));
        }
    }

    let all_var_ids = Arc::new(all_var_ids);
    let mut handles = Vec::with_capacity(NUM_CLIENTS);
    let set_clients = ((NUM_CLIENTS as f64 * SET_CLIENT_RATIO).ceil() as usize).max(1);

    for idx in 0..NUM_CLIENTS {
        let ids = Arc::clone(&all_var_ids);
        let host = host.to_string();
        let is_setter = idx < set_clients;
        let handle = tokio::spawn(async move {
            let client = Client::connect(&host, port, tls).await.expect("client connect");
            let total = ids.len();
            let sample_size = ((total * PCT_TOUCH) / 100).max(1);
            let start = tokio::time::Instant::now();
            while start.elapsed() < Duration::from_secs(RUN_SECS) {
                let (subset, values, pause_ms) = {
                    let mut rng = rand::thread_rng();
                    let subset: Vec<String> = ids
                        .choose_multiple(&mut rng, sample_size)
                        .cloned()
                        .collect();
                    let values: Vec<f64> = subset.iter().map(|_| rng.gen_range(0.0..100.0)).collect();
                    let pause = rng.gen_range(1_000..=5_000);
                    (subset, values, pause)
                };

                if is_setter {
                    let _ = client.set_float_variables(subset.clone(), values, 5_000).await;
                }
                let _ = client.get_values(subset, 5_000).await;

                sleep(Duration::from_millis(pause_ms)).await;
            }
            println!("client {idx} done ({})", if is_setter { "set+get" } else { "get" });
        });
        handles.push(handle);
    }

    futures_util::future::join_all(handles).await;
    Ok(())
}
