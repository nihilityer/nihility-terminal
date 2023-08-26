fn main() {
    tonic_build::configure()
        .compile(&["proto/test.proto"], &["proto"])
        .expect("failed to compile protos");
}