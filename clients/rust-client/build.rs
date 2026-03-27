fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.protoc_arg("--experimental_allow_proto3_optional");
    config.compile_protos(
        &[
            "../../proto/namespace/types.proto",
            "../../proto/namespace/enums.proto",
            "../../proto/namespace/commands.proto",
            "../../proto/namespace/events.proto",
        ],
        &["../../proto/"],
    )?;
    Ok(())
}
