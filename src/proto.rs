pub use build::bazel::remote::execution::v2::*;

pub mod google {
    pub mod longrunning {
        tonic::include_proto!("google.longrunning");
    }
    pub mod bytestream {
        tonic::include_proto!("google.bytestream");
    }
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
}

pub mod build {
    pub mod bazel {
        pub mod semver {
            tonic::include_proto!("build.bazel.semver");
        }
        pub mod remote {
            pub mod execution {
                pub mod v2 {
                    tonic::include_proto!("build.bazel.remote.execution.v2");
                }
            }
        }
    }
}
