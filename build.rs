fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf definitions if proto files exist
    let proto_files = std::fs::read_dir("proto");
    if let Ok(entries) = proto_files {
        let proto_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "proto"))
            .map(|e| e.path())
            .collect();

        if !proto_files.is_empty() {
            prost_build::compile_protos(&proto_files, &["proto"])?;
        }
    }

    Ok(())
}
