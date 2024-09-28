mod proto;
mod types;

mod client;

use anyhow::Result;
use client::*;
use types::*;

#[tokio::main]
async fn main() -> Result<()> {
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
