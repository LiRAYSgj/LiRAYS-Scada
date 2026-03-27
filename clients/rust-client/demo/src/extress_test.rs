use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use log::info;
use rand::{thread_rng, Rng};
use tokio::time::sleep;

use lirays_ws_client::{BooleanVar, Client, FloatVar, IntegerVar, TextVar};
use lirays_ws_client::namespace::VarDataType;

pub async fn run(client: &Client) -> Result<()> {
    // 1) Crear carpetas
    let folder_names: Vec<String> = (0..1000).map(|i| format!("folder_{i}")).collect();
    // if let Err(e) = client.create_folders(folder_names.clone(), Some("/Root".to_string()), 5_000).await {
    //     eprintln!("Error creando folders: {e}");
    // }
    info!("folders creadas");

    // Mapear nombres -> ids listando root
    let (folders_info, _) = client.list(Some("/Root".to_string()), 5_000).await?;
    let id_by_name: HashMap<String, String> = folders_info
        .into_iter()
        .filter_map(|f| {
            if folder_names.contains(&f.name) {
                Some((f.name, f.id))
            } else {
                None
            }
        })
        .collect();

    // Acumuladores de ids por tipo para luego setear
    let mut int_ids: Vec<String> = Vec::new();
    let mut float_ids: Vec<String> = Vec::new();
    let mut text_ids: Vec<String> = Vec::new();
    let mut bool_ids: Vec<String> = Vec::new();

    for (idx, fname) in folder_names.iter().enumerate() {
        let folder_id = match id_by_name.get(fname) {
            Some(id) => id.clone(),
            None => continue, // si no se encontró, saltamos
        };

        // 2) Armar 100 variables repartidas en 4 tipos
        let mut ints: Vec<IntegerVar> = Vec::new();
        let mut floats: Vec<FloatVar> = Vec::new();
        let mut texts: Vec<TextVar> = Vec::new();
        let mut bools: Vec<BooleanVar> = Vec::new();

        for i in 0..1000 {
            let base = format!("{fname}_{i}");
            match i % 4 {
                0 => ints.push(IntegerVar {
                    name: format!("{base}_int"),
                    unit: Some("u".into()),
                    min: Some(0.0),
                    max: Some(100.0),
                }),
                1 => floats.push(FloatVar {
                    name: format!("{base}_flt"),
                    unit: Some("u".into()),
                    min: Some(0.0),
                    max: Some(100.0),
                }),
                2 => texts.push(TextVar {
                    name: format!("{base}_txt"),
                    unit: Some("unit".into()),
                    options: vec!["A".into(), "B".into(), "C".into()],
                    max_len: vec![8],
                }),
                _ => bools.push(BooleanVar {
                    name: format!("{base}_bool"),
                    unit: Some("flag".into()),
                }),
            }
        }

        // if let Err(e) = client
        //     .create_integer_variables(ints, Some(folder_id.clone()), 5_000)
        //     .await
        // {
        //     eprintln!("Error creando ints en {fname}: {e}");
        // }
        // if let Err(e) = client
        //     .create_float_variables(floats, Some(folder_id.clone()), 5_000)
        //     .await
        // {
        //     eprintln!("Error creando floats en {fname}: {e}");
        // }
        // if let Err(e) = client
        //     .create_text_variables(texts, Some(folder_id.clone()), 5_000)
        //     .await
        // {
        //     eprintln!("Error creando texts en {fname}: {e}");
        // }
        // if let Err(e) = client
        //     .create_boolean_variables(bools, Some(folder_id.clone()), 5_000)
        //     .await
        // {
        //     eprintln!("Error creando bools en {fname}: {e}");
        // }
        // info!("variables creadas en {}", fname);

        // 3) Listar la carpeta recién creada para capturar los IDs
        let (_folders_in, vars_in) = match client.list(Some(folder_id.clone()), 5_000).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Error listando {fname}: {e}");
                continue;
            }
        };
        for v in vars_in {
            match VarDataType::try_from(v.var_d_type).unwrap_or(VarDataType::Invalid) {
                VarDataType::Integer => int_ids.push(v.id),
                VarDataType::Float => float_ids.push(v.id),
                VarDataType::Text => text_ids.push(v.id),
                VarDataType::Boolean => bool_ids.push(v.id),
                _ => {}
            }
        }
        info!("capturados IDs de {}", fname);

        // Pequeña pausa para no saturar el server al crear
        if idx % 3 == 0 {
            sleep(Duration::from_millis(200)).await;
        }
    }

    info!(
        "Listo para enviar sets: ints={}, floats={}, texts={}, bools={}",
        int_ids.len(),
        float_ids.len(),
        text_ids.len(),
        bool_ids.len()
    );

    // 4) Loop infinito seteando valores aleatorios cada 1s
    let mut rng = thread_rng();
    loop {
        let int_vals: Vec<i64> = (0..int_ids.len())
            .map(|_| rng.gen_range(0..=100))
            .collect();
        let float_vals: Vec<f64> = (0..float_ids.len())
            .map(|_| rng.gen_range(0.0..=100.0))
            .collect();
        let text_vals: Vec<String> = (0..text_ids.len())
            .map(|_| ["A", "B", "C"][rng.gen_range(0..3)].to_string())
            .collect();
        let bool_vals: Vec<bool> = (0..bool_ids.len())
            .map(|_| rng.gen_bool(0.5))
            .collect();

        if let Err(e) = client
            .set_integer_variables(int_ids.clone(), int_vals, 5_000)
            .await
        {
            eprintln!("Error seteando enteros: {e}");
        }
        if let Err(e) = client
            .set_float_variables(float_ids.clone(), float_vals, 5_000)
            .await
        {
            eprintln!("Error seteando floats: {e}");
        }
        if let Err(e) = client
            .set_text_variables(text_ids.clone(), text_vals, 5_000)
            .await
        {
            eprintln!("Error seteando textos: {e}");
        }
        if let Err(e) = client
            .set_boolean_variables(bool_ids.clone(), bool_vals, 5_000)
            .await
        {
            eprintln!("Error seteando bools: {e}");
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
