use anyhow::Result;
use lirays_ws_client::Client;

/// Bulk create a deep namespace (~10 levels) with ~1k variables using inline JSON.
pub async fn run(host: &str, port: i64, tls: bool) -> Result<()> {
    let client = Client::connect_with_credentials(host, port, tls, "admin", "qwe123").await?;

// Inline schema; expansions target ~1k vars.
    let schema = r#"
    {
      "PlantA": {
        "Area_[:2]": {
          "Line_[:2]": {
            "Section_[A,B,C]": {
              "Panel_[:2]": {
                "Rack_[:2]": {
                  "Slot_[:2]": {
                    "AI_[:3]": {"variable": {"var_d_type": "Float", "unit": "degC", "min": -50, "max": 150, "options": [], "max_len": null}},
                    "AO_[:2]": {"variable": {"var_d_type": "Float", "unit": "%", "min": 0, "max": 100, "options": [], "max_len": null}},
                    "DI_[:2]": {"variable": {"var_d_type": "Boolean", "unit": null, "min": null, "max": null, "options": [], "max_len": null}},
                    "DO_[:2]": {"variable": {"var_d_type": "Boolean", "unit": null, "min": null, "max": null, "options": [], "max_len": null}},
                    "Status_[ok,warn,fail]": {"variable": {"var_d_type": "Text", "unit": null, "min": null, "max": null, "options": ["ok","warn","fail"], "max_len": 8}}
                  }
                }
              }
            }
          }
        }
      }
    }
    "#;

    client.create_bulk_from_json(schema, Some("/".into()), 30_000).await?;
    println!("Bulk creation finished");
    client.disconnect().await?;
    Ok(())
}
