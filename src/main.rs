use github_v3::{Client, GHError};
use serde_derive::*;
use std::cmp::{max, min};
use std::time::Duration;
use std::io::{self, Read};
use std::iter::IntoIterator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Serp {
    pub total_count: u32,
}

#[tokio::main]
async fn main() -> Result<(), GHError> {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        0 | 1 => {
            if atty::is(atty::Stream::Stdin) {
                run(1000..=65535).await
            } else {
                let mut buffer = String::new();
                match io::stdin().read_to_string(&mut buffer) {
                    Ok(n) if n > 0 => {
                        run(buffer.split('\n').flat_map(|n| n.parse::<u32>().ok()).into_iter()).await
                    },
                    _ => run(1000..=65535).await
                }
            }
        },
        2 => run(1000..=args[1].parse().unwrap_or(65535)).await,
        _ => run((args[1].parse().unwrap_or(1000))..=(args[2].parse().unwrap_or(65535))).await
    }
}

async fn run<I>(mut ports: I) -> Result<(), GHError>
where I: Iterator<Item=u32> {
    let gh = Client::new_from_env();
    let min_sleep = 4; // the published rate limit is 30 requests per minute
    let max_sleep = 600;
    let max_errors = 10;

    let mut item = ports.next();
    let mut consecutive_errors = 0;
    let mut next_sleep = 0;
    while let Some(port) = item {
        std::thread::sleep(Duration::from_secs(next_sleep));
        if next_sleep < min_sleep {
            next_sleep = min_sleep;
        }
        let query = format!("\"localhost:{}\"", port);
        let res = gh
            .get()
            .path("search/code")
            .query("q=")
            .arg(&query)
            .send()
            .await;
        if let Ok(res) = res {
            if let Ok(res) = res.obj::<Serp>().await {
                println!("{},{}", port, res.total_count);
                if next_sleep > min_sleep {
                    next_sleep = max(min_sleep, next_sleep / 2);
                }
                consecutive_errors = 0;
                item = ports.next();
                continue;
            }
        }

        if next_sleep < max_sleep {
            if next_sleep < 32 {
                next_sleep = 32
            } else {
                next_sleep = min(max_sleep, next_sleep * 2);
            }
        }
        consecutive_errors += 1;
        eprintln!(
            "[{}] {} consecutive errors. sleeping for {} secs",
            port, consecutive_errors, next_sleep
        );
        if consecutive_errors > max_errors {
            break;
        }
    }
    Ok(())
}