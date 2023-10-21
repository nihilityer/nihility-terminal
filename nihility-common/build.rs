fn main() {
    tonic_build::configure()
        .compile(
            &[
                "proto/manipulate.proto",
                "proto/instruct.proto",
                "proto/module_info.proto",
            ],
            &["proto"],
        )
        .expect("failed to compile protos");
}
