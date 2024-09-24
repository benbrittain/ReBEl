mod proto;

use proto::{execution_client::ExecutionClient, ExecuteRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ExecutionClient::connect("http://[::1]:8980").await?;

    let response = client
        .execute(tonic::Request::new(ExecuteRequest {
            instance_name: "remote-execution".to_string(),
            skip_cache_lookup: false,
            action_digest: None,
            execution_policy: None,
            results_cache_policy: None,
        }))
        .await;

    println!("RESPONSE={:?}", response);

    Ok(())
}
