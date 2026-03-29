use anyhow::Result;
use rand::Rng;
use tokio::time::{sleep, Duration};
use lirays_ws_client::{Client, IntegerVar, FloatVar, TextVar, BooleanVar};

const NUM_CLIENTS: usize = 100;
const MAX_BATCH: usize = 100;
const RUN_SECS: u64 = 60;

pub async fn run(host: &str, port: i64, tls: bool) -> Result<()> {
    let mut handles = Vec::with_capacity(NUM_CLIENTS);
    for idx in 0..NUM_CLIENTS {
        let host = host.to_string();
        let handle = tokio::spawn(async move {
            let client = match Client::connect(&host, port, tls).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("client {idx} connect error: {e}");
                    return;
                }
            };
            let root_name = format!("Root{idx}");
            let root_path = format!("/{}", root_name);
            let _ = client.create_folders(vec![root_name.clone()], None, 5_000).await;

            let start = tokio::time::Instant::now();
            let mut create_phase = true;
            let mut last_created_folders: Vec<String> = Vec::new();
            let mut last_created_vars: Vec<String> = Vec::new();

            while start.elapsed() < Duration::from_secs(RUN_SECS) {
                let sleep_ms = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(1_000..=5_000)
                };
                sleep(Duration::from_millis(sleep_ms)).await;

                if create_phase {
                    // Random counts
                    let (folders_n, vars_n) = {
                        let mut rng = rand::thread_rng();
                        (rng.gen_range(1..=MAX_BATCH), rng.gen_range(1..=MAX_BATCH))
                    };

                    // Create folders under this root
                    let folders: Vec<String> = {
                        let mut rng = rand::thread_rng();
                        (0..folders_n)
                            .map(|i| format!("f{}", rng.r#gen::<u32>() ^ i as u32))
                            .collect()
                    };
                    let _ = client.create_folders(folders.clone(), Some(root_path.clone()), 5_000).await;
                    last_created_folders = folders.iter().map(|name| format!("{}/{}", root_path, name)).collect();

                    // Create variables under root
                    let mut int_vars = Vec::new();
                    let mut float_vars = Vec::new();
                    let mut text_vars = Vec::new();
                    let mut bool_vars = Vec::new();
                    for i in 0..vars_n {
                        match i % 4 {
                            0 => int_vars.push(IntegerVar {
                                name: format!("vi_{}", rand::thread_rng().r#gen::<u32>() ^ i as u32),
                                unit: None,
                                min: None,
                                max: None,
                            }),
                            1 => float_vars.push(FloatVar {
                                name: format!("vf_{}", rand::thread_rng().r#gen::<u32>() ^ i as u32),
                                unit: None,
                                min: None,
                                max: None,
                            }),
                            2 => text_vars.push(TextVar {
                                name: format!("vt_{}", rand::thread_rng().r#gen::<u32>() ^ i as u32),
                                unit: None,
                                options: vec!["A".into(), "B".into(), "C".into()],
                                max_len: Some(8),
                            }),
                            _ => bool_vars.push(BooleanVar {
                                name: format!("vb_{}", rand::thread_rng().r#gen::<u32>() ^ i as u32),
                                unit: None,
                            }),
                        }
                    }
                    // Track created var ids for deletion phase
                    last_created_vars = int_vars.iter().map(|v| format!("{}/{}", root_path, v.name.clone()))
                        .chain(float_vars.iter().map(|v| format!("{}/{}", root_path, v.name.clone())))
                        .chain(text_vars.iter().map(|v| format!("{}/{}", root_path, v.name.clone())))
                        .chain(bool_vars.iter().map(|v| format!("{}/{}", root_path, v.name.clone())))
                        .collect();

                    if !int_vars.is_empty() {
                        let _ = client.create_integer_variables(int_vars, Some(root_path.clone()), 5_000).await;
                    }
                    if !float_vars.is_empty() {
                        let _ = client.create_float_variables(float_vars, Some(root_path.clone()), 5_000).await;
                    }
                    if !text_vars.is_empty() {
                        let _ = client.create_text_variables(text_vars, Some(root_path.clone()), 5_000).await;
                    }
                    if !bool_vars.is_empty() {
                        let _ = client.create_boolean_variables(bool_vars, Some(root_path.clone()), 5_000).await;
                    }

                    // Trigger list on current root to record list metrics
                    let _ = client.list(Some(root_path.clone()), 5_000).await;
                    // If we created folders, also list the first one
                    if let Some(first_folder) = last_created_folders.first() {
                        let _ = client.list(Some(first_folder.clone()), 5_000).await;
                    }
                } else {
                    // Delete everything created in previous create phase, except keep first folder and first var
                    if last_created_vars.len() > 1 {
                        let to_delete: Vec<String> = last_created_vars.iter().skip(1).cloned().collect();
                        if !to_delete.is_empty() {
                            let _ = client.delete_items(to_delete, 5_000).await;
                        }
                        // keep last_created_vars[0]
                        last_created_vars = vec![last_created_vars[0].clone()];
                    }
                    if last_created_folders.len() > 1 {
                        let to_delete: Vec<String> = last_created_folders.iter().skip(1).cloned().collect();
                        if !to_delete.is_empty() {
                            let _ = client.delete_items(to_delete, 5_000).await;
                        }
                        last_created_folders = vec![last_created_folders[0].clone()];
                    }

                    // List root and the remaining folder (if any) to capture list performance too
                    let _ = client.list(Some(root_path.clone()), 5_000).await;
                    if let Some(first_folder) = last_created_folders.first() {
                        let _ = client.list(Some(first_folder.clone()), 5_000).await;
                    }
                }

                create_phase = !create_phase;
            }
        });
        handles.push(handle);
    }

    futures_util::future::join_all(handles).await;
    Ok(())
}
