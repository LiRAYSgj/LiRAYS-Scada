use std::time::Duration;

use anyhow::Result;
use log::{info, warn};
use rand::{thread_rng, Rng};
use tokio::time::sleep;
use futures_util::future;

use lirays_ws_client::{Client, FloatVar};

const NUM_CLIENTS: usize = 10;
const VARS_PER_CLIENT: usize = 1000;
const ROOT: &str = "/Root";

pub async fn setup_namespace(host: &str, port: i64, tls: bool) -> Result<()> {
    let client = Client::connect(host, port, tls).await?;
    info!("setup: conectado");

    // Crear carpeta Root si no existe (usamos add bajo /)
    let folder_names: Vec<String> = (0..NUM_CLIENTS).map(|i| format!("thread_{i}")).collect();
    client
        .create_folders(folder_names.clone(), Some(ROOT.to_string()), 30_000)
        .await?;
    info!("setup: carpetas creadas");

    // Crear 1000 floats por carpeta, en chunks de 200
    for fname in folder_names {
        let parent = format!("{ROOT}/{}", fname);
        let mut vars = Vec::with_capacity(VARS_PER_CLIENT);
        for i in 0..VARS_PER_CLIENT {
            vars.push(FloatVar {
                name: format!("var_{i}_float"),
                unit: Some("u".into()),
                min: Some(0.0),
                max: Some(100.0),
            });
        }
        for chunk in vars.chunks(200) {
            client
                .create_float_variables(chunk.to_vec(), Some(parent.clone()), 30_000)
                .await?;
        }
        info!("setup: variables creadas en {}", parent);
    }

    Ok(())
}

pub async fn run(host: &str, port: i64, tls: bool) -> Result<()> {
    // 1000 hilos (conexiones), cada uno escribe 1000 variables
    let mut handles = Vec::with_capacity(NUM_CLIENTS);

    for idx in 0..NUM_CLIENTS {
        let h = host.to_string();
        let vars = VARS_PER_CLIENT;
        let handle = tokio::spawn(async move {
            let client = match Client::connect(&h, port, tls).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("client {idx}: error conectando: {e}");
                    return;
                }
            };
            info!("client {idx} conectado");

            // Preconstruye ids: /Root/thread_{idx}/var_{i}_int
            let ids: Vec<String> = (0..vars)
                .map(|i| format!("{ROOT}/thread_{idx}/var_{i}_float"))
                .collect();

            loop {
                // Enviar en batch completo (1000) para este cliente
                let vals: Vec<i64> = {
                    let mut rng = thread_rng();
                    (0..ids.len()).map(|_| rng.gen_range(0..=100)).collect()
                };
                if let Err(e) = client
                    .set_float_variables(ids.clone(), vals.into_iter().map(|v| v as f64).collect(), 15_000)
                    .await
                {
                    warn!("client {idx}: error seteando floats: {e}");
                }
                sleep(Duration::from_millis(1000)).await;
            }
        });
        handles.push(handle);
    }

    // Mantener vivo
    future::join_all(handles).await;
    Ok(())
}
