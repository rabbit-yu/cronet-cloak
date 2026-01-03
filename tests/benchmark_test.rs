use cronet_cloak::cronet::CronetEngine;
use cronet_cloak::cronet_pb::{ExecutionConfig, TargetRequest};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;

// Helper to execute request via engine
async fn execute_engine_request(engine: &Arc<CronetEngine>, url: &str) {
    let target = TargetRequest {
        url: url.to_string(),
        method: "GET".to_string(),
        headers: Default::default(),
        body: vec![],
    };
    let config = ExecutionConfig::default();

    let (handle, rx) = engine.start_request(&target, &config);
    let _ = rx.await;
    // Ensure handle is dropped
    drop(handle);
}

#[tokio::test]
async fn benchmark_single_get() {
    let engine = Arc::new(CronetEngine::new("Benchmark/1.0"));
    let n = 10; // Reduced from benchmark levels for CI/Test speed

    let start = Instant::now();
    for _ in 0..n {
        execute_engine_request(&engine, "https://httpbin.org/uuid").await;
    }
    let duration = start.elapsed();
    println!(
        "BenchmarkSingleGet: {:?} for {} reqs ({:?}/req)",
        duration,
        n,
        duration / n as u32
    );
}

#[tokio::test]
async fn benchmark_parallel_get() {
    let engine = Arc::new(CronetEngine::new("Benchmark/1.0"));
    let n = 20;
    let concurrency = 5;
    let sem = Arc::new(Semaphore::new(concurrency));

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..n {
        let engine = engine.clone();
        let sem = sem.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            execute_engine_request(&engine, "https://httpbin.org/uuid").await;
        }));
    }

    for h in handles {
        let _ = h.await;
    }
    let duration = start.elapsed();
    println!(
        "BenchmarkParallelGet: {:?} for {} reqs (Concurrent {})",
        duration, n, concurrency
    );
}

#[tokio::test]
async fn benchmark_connection_reuse() {
    let engine = Arc::new(CronetEngine::new("Benchmark/1.0"));
    let n = 20;

    let start = Instant::now();
    for _ in 0..n {
        // httpbin/status/200 returns immediately
        execute_engine_request(&engine, "https://httpbin.org/status/200").await;
    }
    let duration = start.elapsed();
    println!("BenchmarkConnectionReuse: {:?} for {} reqs", duration, n);
}

// Payload Benchmarks
#[tokio::test]
async fn benchmark_payloads() {
    let engine = Arc::new(CronetEngine::new("Benchmark/1.0"));
    let n = 5; // Expensive

    let scenarios = vec![
        ("Small", "https://httpbin.org/bytes/100"),
        ("Medium", "https://httpbin.org/bytes/10240"),
        ("Large", "https://httpbin.org/bytes/102400"),
    ];

    for (name, url) in scenarios {
        let start = Instant::now();
        for _ in 0..n {
            execute_engine_request(&engine, url).await;
        }
        let duration = start.elapsed();
        println!("BenchmarkPayload{}: {:?} for {} reqs", name, duration, n);
    }
}

// Memory stability test (simplified)
#[tokio::test]
async fn test_engine_stability_repeated() {
    let engine = Arc::new(CronetEngine::new("Benchmark/Stability"));
    let n = 50;

    // Just run loop, verify no panic
    for i in 0..n {
        execute_engine_request(&engine, "https://example.com/").await;
        if i % 10 == 0 {
            // "GC" not relevant, but explicit drop happens
        }
    }
    println!("Stability test passed ({} requests)", n);
}
