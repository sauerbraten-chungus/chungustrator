fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(
        &[
            "proto/chungustrator_enet.proto",
            "proto/chungustrator.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
