use reqwest::Client;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::Instant;

const TOTAL_REQUESTS: usize = 2000;
const CONCURRENT_WORKERS: usize = 20;
const SERVICE_URL: &str = "http://127.0.0.1:3000/api/execute";

#[tokio::test]
async fn test_concurrent_proxy_requests() {
    let client = Client::builder()
        .pool_max_idle_per_host(CONCURRENT_WORKERS)
        .build()
        .unwrap();

    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));
    let start_time = Instant::now();

    let mut tasks = Vec::new();
    let requests_per_worker = TOTAL_REQUESTS / CONCURRENT_WORKERS;

    // Spawn workers with dedicated ranges
    for worker_id in 0..CONCURRENT_WORKERS {
        let client = client.clone();
        let success_count = success_count.clone();
        let failure_count = failure_count.clone();

        let start_idx = worker_id * requests_per_worker;
        let end_idx = if worker_id == CONCURRENT_WORKERS - 1 {
            TOTAL_REQUESTS
        } else {
            (worker_id + 1) * requests_per_worker
        };

        tasks.push(tokio::spawn(async move {
            for req_id in start_idx..end_idx {
                // Construct Request
                let payload = json!({
                    "request_id": format!("req-{}", req_id),
                    "target": {
                        "url": "https://httpbin.org/ip",
                        "method": "GET"
                    },
                    "config": {
                       "follow_redirects": true
                    }
                });

                let resp = client.post(SERVICE_URL).json(&payload).send().await;

                match resp {
                    Ok(r) => {
                        if r.status().is_success() {
                            let body: serde_json::Value = r.json().await.unwrap_or(json!({}));
                            if body
                                .get("success")
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false)
                            {
                                success_count.fetch_add(1, Ordering::Relaxed);
                            } else {
                                failure_count.fetch_add(1, Ordering::Relaxed);
                            }
                        } else {
                            // Httpbin might rate limit or network fail
                            failure_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        failure_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    // Wait for all workers
    for task in tasks {
        let _ = task.await;
    }

    let duration = start_time.elapsed();
    let successes = success_count.load(Ordering::Relaxed);
    let failures = failure_count.load(Ordering::Relaxed);

    println!("Total Requests: {}", TOTAL_REQUESTS);
    println!("Success: {}", successes);
    println!("Failed: {}", failures);
    println!("Duration: {:?}", duration);
    println!(
        "RPS: {:.2}",
        (TOTAL_REQUESTS as f64) / duration.as_secs_f64()
    );

    // Allow some failures due to external service (httpbin) rate limiting/flakiness
    // But check that at least SOME succeeded to verify parsing/execution.
    assert!(successes > 0, "Zero successes - integration setup failure?");

    // Warn if failure rate is high
    if failures > 0 {
        println!("WARNING: {} requests failed.", failures);
    }
}
