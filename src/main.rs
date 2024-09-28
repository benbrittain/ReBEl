mod proto;
mod types;

mod client;

use anyhow::Result;
use clap::Parser;
use client::*;
use futures::stream::{self, StreamExt};
use proto::ActionResult;
use types::*;

/// RBE load generator
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Max number of requests in-flight
    #[arg(short, long, default_value_t = 1)]
    max_requests: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let fut = move || async {
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
        Ok::<ActionResult, anyhow::Error>(response)
    };

    // Create an infinite unordered buffer of action exections
    let mut actions = stream::repeat_with(|| fut()).buffer_unordered(args.max_requests);

    while let Some(result) = actions.next().await {
        println!("RESPONSE={:#?}", result);
    }

    Ok(())
}
