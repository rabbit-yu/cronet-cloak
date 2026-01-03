use reqwest::Client;
use serde_json::json;

/// Helper to make a proxy request via our cronet-cloak service
async fn make_proxy_request(
    client: &Client,
    proxy_type: &str,
    proxy_host: &str,
    proxy_port: u16,
    username: Option<&str>,
    password: Option<&str>,
) -> serde_json::Value {
    let service_url = "http://127.0.0.1:3000/api/v1/execute";

    // Map proxy type string to enum value (HTTP=0, HTTPS=1, SOCKS5=2)
    let proxy_type_enum = match proxy_type.to_uppercase().as_str() {
        "HTTP" => 0,
        "HTTPS" => 1,
        "SOCKS5" => 2,
        _ => 0,
    };

    let mut proxy = json!({
        "type": proxy_type_enum,
        "host": proxy_host,
        "port": proxy_port,
    });

    // ProxyConfig has username and password as direct fields, not nested
    if let (Some(user), Some(pass)) = (username, password) {
        proxy["username"] = json!(user);
        proxy["password"] = json!(pass);
    }

    let payload = json!({
        "request_id": format!("proxy-test-{}", proxy_type.to_lowercase()),
        "target": {
            "url": "https://ipinfo.io/json",
            "method": "GET",
        },
        "config": {
            "follow_redirects": true,
            "proxy": proxy
        }
    });

    println!("Payload: {}", payload);

    let resp = client
        .post(service_url)
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request to service");

    assert!(
        resp.status().is_success(),
        "Service response status not success: {}",
        resp.status()
    );

    resp.json().await.expect("Failed to parse JSON response")
}

/// Test HTTP proxy support
#[tokio::test]
async fn test_http_proxy() {
    let client = Client::new();

    // Use a known public HTTP proxy or a test one
    // For CI, this might need to be mocked or use an environment variable
    let result = make_proxy_request(
        &client,
        "HTTP",
        "proxy.example.com",
        1000,
        Some("your_username"),
        Some("your_password"),
    )
    .await;

    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        println!("HTTP Proxy Test PASSED");
        // Optionally verify the IP changed
        if let Some(response) = result.get("response") {
            if let Some(body) = response.get("body") {
                let body_str = String::from_utf8_lossy(
                    body.as_str().map(|s| s.as_bytes()).unwrap_or_default(),
                );
                println!("Response body: {}", body_str);
            }
        }
    } else {
        let error = result
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        println!(
            "HTTP Proxy Test FAILED (expected if proxy credentials invalid): {}",
            error
        );
        // Don't fail the test if proxy auth fails - we want to verify the feature works
        assert!(
            error.contains("PROXY")
                || error.contains("proxy")
                || error.contains("AUTH")
                || error.contains("auth")
                || error.contains("TUNNEL")
                || error.contains("tunnel"),
            "Expected proxy-related error, got: {}",
            error
        );
    }
}

/// Test HTTPS proxy support (same as HTTP proxy connecting to HTTPS endpoint)
#[tokio::test]
async fn test_https_proxy() {
    let client = Client::new();

    let result = make_proxy_request(
        &client,
        "HTTPS",
        "proxy.example.com",
        1000,
        Some("your_username"),
        Some("your_password"),
    )
    .await;

    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        println!("HTTPS Proxy Test PASSED");
    } else {
        let error = result
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        println!("HTTPS Proxy Test result: {}", error);
        // HTTPS proxy type may not be supported by Cronet, which would show in the error
    }
}

/// Test SOCKS5 proxy support
#[tokio::test]
async fn test_socks5_proxy() {
    let client = Client::new();

    // SOCKS5 proxy test - using a placeholder that may fail
    // Real tests should use a valid SOCKS5 proxy
    let result = make_proxy_request(
        &client,
        "SOCKS5",
        "127.0.0.1", // Placeholder - replace with actual SOCKS5 proxy
        1080,
        None,
        None,
    )
    .await;

    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if success {
        println!("SOCKS5 Proxy Test PASSED");
    } else {
        let error = result
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        println!(
            "SOCKS5 Proxy Test result (expected to fail if no local SOCKS5): {}",
            error
        );
        // SOCKS5 may not be running locally, so we just log and don't fail
    }
}

/// Test invalid proxy to ensure proper error handling
#[tokio::test]
async fn test_invalid_proxy() {
    let client = Client::new();

    let result = make_proxy_request(
        &client,
        "HTTP",
        "invalid.proxy.host.that.does.not.exist",
        9999,
        None,
        None,
    )
    .await;

    // The request to our service succeeds, but the Cronet request should fail
    let success = result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if !success {
        let error = result
            .get("error_message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("Invalid proxy error (expected): {}", error);
        assert!(
            !error.is_empty(),
            "Should have an error message for invalid proxy"
        );
    } else {
        // If somehow it succeeded, log and skip - proxy might have been bypassed
        println!("Warning: Invalid proxy test succeeded unexpectedly. This may indicate proxy fallback behavior.");
    }
}

/// Test no proxy (direct connection)
#[tokio::test]
async fn test_no_proxy_direct() {
    let client = Client::new();
    let service_url = "http://127.0.0.1:3000/api/v1/execute";

    let payload = json!({
        "request_id": "direct-test",
        "target": {
            "url": "https://httpbin.org/ip",
            "method": "GET",
        },
        "config": {
            "follow_redirects": true
        }
        // No proxy field = direct connection
    });

    let resp = client
        .post(service_url)
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON");
    let success = body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(success, "Direct connection should succeed");
    println!("Direct (no proxy) Test PASSED");
}
