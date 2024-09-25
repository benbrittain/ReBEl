mod proto;
use prost::Message;
use tokio_stream::StreamExt;

use proto::{
    content_addressable_storage_client::ContentAddressableStorageClient,
    execution_client::ExecutionClient, Action, ActionResult, BatchUpdateBlobsRequest, Command,
    Digest, ExecuteRequest, ExecuteResponse,
};

use anyhow::{anyhow, Result};

struct RebelClient {
    cas_client: ContentAddressableStorageClient<tonic::transport::Channel>,
    execute_client: ExecutionClient<tonic::transport::Channel>,
    instance_name: String,
}

struct Blob {
    pub inner: Vec<u8>,
    pub digest: Digest,
}

impl Blob {
    fn from_proto<T: prost::Message>(data: T) -> Blob {
        let data = data.encode_to_vec();
        Blob::new(&data)
    }

    fn new(data: &[u8]) -> Blob {
        let one_shot = ring::digest::digest(&ring::digest::SHA256, data);
        let digest = hex::encode(one_shot);
        Blob {
            inner: data.into(),
            digest: Digest {
                hash: digest,
                size_bytes: data.len() as i64,
            },
        }
    }
}

impl RebelClient {
    pub async fn new(instance_name: &str) -> Result<RebelClient> {
        let execute_client = ExecutionClient::connect("http://[::1]:8980").await?;
        let cas_client = ContentAddressableStorageClient::connect("http://[::1]:8980").await?;
        Ok(RebelClient {
            instance_name: instance_name.to_string(),
            cas_client,
            execute_client,
        })
    }

    pub async fn upload_blob(&mut self, blob: Blob) -> Result<Digest> {
        let batch_update_blobs_resp = self
            .cas_client
            .batch_update_blobs(BatchUpdateBlobsRequest {
                instance_name: "remote-execution".to_string(),
                requests: vec![proto::batch_update_blobs_request::Request {
                    digest: Some(blob.digest),
                    data: blob.inner,
                    compressor: 0,
                }],
            })
            .await?;

        let resp = &batch_update_blobs_resp.get_ref().responses[0];
        let command_digest = resp.digest.as_ref().unwrap();
        Ok(command_digest.clone())
    }

    pub async fn upload_directory(&mut self, dir: Directory) -> Result<Digest> {
        let mut proto = proto::Directory::default();
        for file in dir.files {
            let file_contents = Blob::new(&file.data);
            let file_contents_digest = self.upload_blob(file_contents).await?;
            proto.files.push(proto::FileNode {
                name: file.name,
                digest: Some(file_contents_digest),
                is_executable: file.is_executable,
                node_properties: None,
            })
        }
        self.upload_blob(Blob::from_proto(proto)).await
    }

    pub async fn execute_action(&mut self, action: RebelAction) -> Result<ActionResult> {
        let command = Command {
            arguments: action.args,
            environment_variables: vec![],
            output_paths: action.output_paths,
            working_directory: action.working_directory,
            output_node_properties: vec![],
            // deprecated
            platform: None,
            output_directories: vec![],
            output_files: vec![],
        };
        let command_digest = self.upload_blob(Blob::from_proto(command)).await?;

        let input_root = action.input_root.expect("Input root must be set");
        let input_root_digest = self.upload_directory(input_root).await?;

        let action = Action {
            command_digest: Some(command_digest.clone()),
            input_root_digest: Some(input_root_digest.clone()),
            do_not_cache: true,
            platform: None,
            salt: vec![],
            timeout: None,
        };
        let action_digest = self.upload_blob(Blob::from_proto(action)).await?;

        let request = self
            .execute_client
            .execute(tonic::Request::new(ExecuteRequest {
                instance_name: self.instance_name.clone(),
                skip_cache_lookup: false,
                action_digest: Some(action_digest),
                execution_policy: None,
                results_cache_policy: None,
            }))
            .await?;

        let mut stream = request.into_inner();
        while let Some(op) = stream.next().await {
            let op = op?;
            if op.done {
                use proto::google::longrunning::operation::Result;
                match op.result {
                    Some(Result::Response(prost_types::Any { type_url, value })) => {
                        assert_eq!(
                            type_url,
                            "type.googleapis.com/build.bazel.remote.execution.v2.ExecuteResponse"
                        );
                        let execute_response = ExecuteResponse::decode(&*value)?;
                        assert_eq!(execute_response.status, None);
                        return Ok(execute_response
                            .result
                            .expect("result field should be populated"));
                    }
                    Some(_) | None => todo!(),
                }
            }
        }
        unreachable!();
    }
}

#[derive(Default)]
struct RebelAction {
    args: Vec<String>,
    working_directory: String,
    input_root: Option<Directory>,
    output_paths: Vec<String>,
}

impl RebelAction {
    // Set the arguments to the command. The first argument specifies the command to
    // run, which may be either an absolute path, a path relative to the working
    // directory, or an unqualified path (without path separators) which will be
    // resolved using the operating system's equivalent of the PATH environment variable.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Sets the working directory of the command.
    pub fn working_dir(mut self, working_directory: String) -> Self {
        self.working_directory = working_directory;
        self
    }

    /// Sets the input root working directory of the command.
    pub fn input_root(mut self, working_directory: Directory) -> Self {
        self.input_root = Some(working_directory);
        self
    }

    /// Sets the input root working directory of the command.
    pub fn output_paths(mut self, output_paths: Vec<String>) -> Self {
        self.output_paths = output_paths;
        self
    }
}

struct Directory {
    name: String,
    directories: Vec<Directory>,
    files: Vec<File>,
}

impl Directory {
    pub fn new(name: String, directories: Vec<Directory>, files: Vec<File>) -> Self {
        Directory {
            name,
            directories,
            files,
        }
    }
}

struct File {
    name: String,
    data: Vec<u8>,
    is_executable: bool,
}

impl File {
    pub fn new(name: String, data: Vec<u8>) -> Self {
        File {
            name,
            data,
            is_executable: false,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = RebelClient::new("remote-execution").await?;

    let action = RebelAction::default()
        .args(vec![
            String::from("/bin/cp"),
            String::from("README"),
            String::from("bwb-test"),
        ])
        .input_root(Directory::new(
            "".to_string(),
            vec![],
            vec![File::new("README".to_string(), b"Hello".to_vec())],
        ))
        .output_paths(vec!["bwb-test".to_string()]);
    let response = client.execute_action(action).await?;

    println!("RESPONSE={:#?}", response);

    Ok(())
}
