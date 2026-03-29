use lirays_ws_client::{BooleanVar, Client, FloatVar, IntegerVar, TextVar};

pub async fn run(host: &str, port: i64, tls: bool) {
    let client = Client::connect(host, port, tls).await.unwrap();

    // Create folders
    client.create_folders(vec!["demo".into()], None, 5_000).await.unwrap();
    client.create_folders(vec!["sub".into()], Some("/demo".into()), 5_000).await.unwrap();

    // Create vars under /demo
    client
        .create_integer_variables(
            vec![IntegerVar {
                name: "i1".into(),
                unit: Some("u".into()),
                min: Some(0.0),
                max: Some(100.0),
            }],
            Some("/demo".into()),
            5_000,
        )
        .await
        .unwrap();

    client
        .create_float_variables(
            vec![FloatVar {
                name: "f1".into(),
                unit: None,
                min: None,
                max: None,
            }],
            Some("/demo".into()),
            5_000,
        )
        .await
        .unwrap();

    client
        .create_text_variables(
            vec![TextVar {
                name: "t1".into(),
                unit: None,
                options: vec!["A".into(), "B".into()],
                max_len: Some(8),
            }],
            Some("/demo".into()),
            5_000,
        )
        .await
        .unwrap();

    client
        .create_boolean_variables(
            vec![BooleanVar {
                name: "b1".into(),
                unit: None,
            }],
            Some("/demo".into()),
            5_000,
        )
        .await
        .unwrap();

    // List
    let (folders, vars) = client.list(Some("/demo".into()), 5_000).await.unwrap();
    println!("Folders: {:?}", folders);
    println!("Vars: {:?}", vars);

    // Set values
    client
        .set_integer_variables(vec!["/demo/i1".into()], vec![42], 5_000)
        .await
        .unwrap();
    client
        .set_float_variables(vec!["/demo/f1".into()], vec![3.14], 5_000)
        .await
        .unwrap();
    client
        .set_text_variables(vec!["/demo/t1".into()], vec!["A".into()], 5_000)
        .await
        .unwrap();
    client
        .set_boolean_variables(vec!["/demo/b1".into()], vec![true], 5_000)
        .await
        .unwrap();

    // Get values
    let vals = client
        .get_values(
            vec![
                "/demo/i1".into(),
                "/demo/f1".into(),
                "/demo/t1".into(),
                "/demo/b1".into(),
            ],
            5_000,
        )
        .await
        .unwrap();
    println!("Values: {:?}", vals);

    print!("\x07");
}
