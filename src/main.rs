mod proto;
use prost::Message;
use tokio_stream::StreamExt;

use proto::{
    content_addressable_storage_client::ContentAddressableStorageClient,
    execution_client::ExecutionClient, Action, ActionResult, BatchUpdateBlobsRequest, Command,
    Digest, Directory, ExecuteRequest, ExecuteResponse,
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
    fn from_message<T: prost::Message>(data: T) -> Blob {
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

    pub async fn execute_action(&mut self, action_digest: Digest) -> Result<ActionResult> {
        let mut request = self
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = RebelClient::new("remote-execution").await?;

    let blob = Blob::new(b"aeuutest data");

    let command = Command {
        arguments: vec!["/bin/true".to_string()],
        environment_variables: vec![],
        output_paths: vec![],
        working_directory: "".to_string(),
        output_node_properties: vec![],
        // deprecated
        platform: None,
        output_directories: vec![],
        output_files: vec![],
    };
    let command_digest = client.upload_blob(Blob::from_message(command)).await?;
    dbg!(&command_digest);

    let directory = Directory {
        files: vec![],
        directories: vec![],
        symlinks: vec![],
        node_properties: None,
    };
    let input_root_digest = client.upload_blob(Blob::from_message(directory)).await?;

    let action = Action {
        command_digest: Some(command_digest.clone()),
        input_root_digest: Some(input_root_digest.clone()),
        do_not_cache: false,
        platform: None,
        salt: vec![],
        timeout: None,
    };

    let action_digest = client.upload_blob(Blob::from_message(action)).await?;
    dbg!(&action_digest);

    let response = client.execute_action(action_digest).await?;

    println!("RESPONSE={:#?}", response);

    Ok(())
}
