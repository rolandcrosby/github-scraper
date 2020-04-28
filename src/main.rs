use github_v3::*;
use serde_derive::*;
use std::cmp::{max, min};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Serp {
    pub total_count: u32,
}

#[tokio::main]
async fn main() -> Result<(), GHError> {
    let args: Vec<String> = std::env::args().collect();
    let start: u32 = match args.len() {
        1 => 1000,
        _ => args[1].parse().unwrap_or(1000),
    };
    let end: u32 = match args.len() {
        1 | 2 => 65535,
        _ => args[2].parse().unwrap_or(65535),
    };
    let gh = Client::new_from_env();
    let min_sleep = 4; // the published rate limit is 30 requests per minute
    let max_sleep = 600;
    let max_errors = 10;
    let mut port = start;
    let mut next_sleep = 2;
    let mut consecutive_errors = 0;
    while port <= end {
        if port > 1000 || consecutive_errors > 0 {
            std::thread::sleep(Duration::from_secs(next_sleep));
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
                println!("{},{}", query, res.total_count);
                if next_sleep > min_sleep {
                    next_sleep = max(min_sleep, next_sleep / 2);
                }
                consecutive_errors = 0;
                port += 1;
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
            "{} consecutive errors. sleeping for {} secs",
            consecutive_errors, next_sleep
        );
        if consecutive_errors > max_errors {
            break;
        }
    }
    Ok(())
}
