fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(false).compile(
        &[
            "proto/build/bazel/remote/execution/v2/remote_execution.proto",
            "proto/build/bazel/semver/semver.proto",
            "proto/google/api/annotations.proto",
            "proto/google/api/client.proto",
            "proto/google/api/http.proto",
            "proto/google/bytestream/bytestream.proto",
            "proto/google/longrunning/operations.proto",
            "proto/google/rpc/code.proto",
            "proto/google/rpc/error_details.proto",
            "proto/google/rpc/status.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
