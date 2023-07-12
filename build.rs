fn main() {
    prost_build::compile_protos(
        &["protobuf/requests.proto", "protobuf/results.proto"],
        &["protobuf"]
    ).expect("Failed to build protobuf.")
}