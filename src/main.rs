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

    /// URL of RBE service e.g. http://[::1]:8980
    #[arg(short, long)]
    url: String,

    /// Namespace of RBE
    #[arg(short, long, default_value = "remote-execution")]
    namespace: String,

    /// Load scenario
    #[arg(short, long)]
    scenario: Scenario,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum Scenario {
    /// Generate a lot of small files that are read as part of the action.
    ManySmallFiles,
    /// Generate a few of large files that are read as part of the action.
    FewLargeFiles,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let fut = move |scenario: Scenario| {
        let url = args.url.clone();
        let namespace = args.namespace.clone();

        async move {
            let mut client = RebelClient::new(url, &namespace).await?;

            let action = match scenario {
                Scenario::ManySmallFiles => RebelAction::default()
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
                    .output_paths(vec!["bwb-test".to_string()]),
                Scenario::FewLargeFiles => todo!(),
            };

            let response = client.execute_action(action).await?;
            Ok::<ActionResult, anyhow::Error>(response)
        }
    };

    // Create an infinite unordered buffer of action exections
    let mut actions =
        stream::repeat_with(|| fut(args.scenario)).buffer_unordered(args.max_requests);

    while let Some(result) = actions.next().await {
        println!("RESPONSE={:#?}", result);
    }

    Ok(())
}
