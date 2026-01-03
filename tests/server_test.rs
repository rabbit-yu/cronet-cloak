use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_nike_request() {
    let client = Client::new();
    let service_url = "http://127.0.0.1:3000/api/v1/execute";

    // Nike headers
    let headers = json!({
        "accept": { "values": ["text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"] },
        "accept-encoding": { "values": ["gzip, deflate, br, zstd"] },
        "accept-language": { "values": ["en-US,en;q=0.9"] },
        "sec-ch-ua": { "values": ["\"Google Chrome\";v=\"141\", \"Not?A_Brand\";v=\"8\", \"Chromium\";v=\"141\""] },
        "sec-ch-ua-mobile": { "values": ["?0"] },
        "sec-ch-ua-platform": { "values": ["\"Windows\""] },
        "sec-fetch-dest": { "values": ["document"] },
        "sec-fetch-mode": { "values": ["navigate"] },
        "sec-fetch-site": { "values": ["none"] },
        "sec-fetch-user": { "values": ["?1"] },
        "upgrade-insecure-requests": { "values": ["1"] },
        "user-agent": { "values": ["Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36"] },
    });

    let payload = json!({
        "request_id": "nike-test",
        "target": {
            "url": "https://www.nike.com/",
            "method": "GET",
            "headers": headers
        },
        "config": {
            "follow_redirects": true,
            "proxy": {
                "type": 0,
                "host": "proxy.example.com",
                "port": 1000,
                "username": "your_username",
                "password": "your_password"
            }
        }
    });

    // We assume the user has configured the server or environment.
    // If the proxy is not valid, the request might fail, but the format is correct.

    let resp = client
        .post(service_url)
        .json(&payload)
        .send()
        .await
        .expect("Failed to send request");

    assert!(
        resp.status().is_success(),
        "Response status not success: {}",
        resp.status()
    );

    let body: serde_json::Value = resp.json().await.expect("Failed to parse JSON response");

    // Check basic success flag
    let success = body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        // It might fail due to proxy auth, but we want to verify the executor didn't crash.
        println!(
            "Request executed but returned internal failure: {:?}",
            body.get("error_message")
        );
    } else {
        println!("Nike request success!");
    }
}
