fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compila el archivo .proto ubicado en la carpeta proto/
    prost_build::compile_protos(
        &[
            "proto/namespace/types.proto",
            "proto/namespace/enums.proto",
            "proto/namespace/commands.proto"
        ],
        &["proto/"]
    )?;
    Ok(())
}
