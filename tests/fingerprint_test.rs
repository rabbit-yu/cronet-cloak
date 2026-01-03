use cronet_cloak::cronet::CronetEngine;
use cronet_cloak::cronet_pb::{ExecutionConfig, TargetRequest};
use std::sync::Arc;

// Helper to execute request via engine
async fn execute_request(engine: &Arc<CronetEngine>, url: &str) -> serde_json::Value {
    let target = TargetRequest {
        url: url.to_string(),
        method: "GET".to_string(),
        headers: Default::default(),
        body: vec![],
    };
    let config = ExecutionConfig::default();

    let (_handle, rx) = engine.start_request(&target, &config);
    let result = rx.await.expect("Channel closed").expect("Request failed");

    // Parse body as string then JSON
    let body_str = String::from_utf8(result.body).expect("Body not UTF-8");
    serde_json::from_str(&body_str).expect("Failed to parse JSON")
}

#[tokio::test]
async fn test_tls_fingerprint() {
    let engine = Arc::new(CronetEngine::new("FingerprintTest/1.0"));

    eprintln!("Sending request to tls.peet.ws...");
    let json = execute_request(&engine, "https://tls.peet.ws/api/all").await;

    // Check HTTP2
    let http2 = json.get("http2").expect("Missing http2 field");
    println!("\n=== HTTP2 Fingerprint ===");
    println!("{}", serde_json::to_string_pretty(http2).unwrap());

    // Check Peetprint
    // Check Peetprint
    let tls = json.get("tls").expect("Missing tls field");
    let peetprint = match tls.get("peetprint") {
        Some(p) => p,
        None => {
            println!(
                "\n[WARN] 'peetprint' field missing in 'tls'. Available keys: {:?}",
                tls.as_object().unwrap().keys()
            );
            &serde_json::Value::Null
        }
    };

    if !peetprint.is_null() {
        let got = peetprint.as_str().unwrap_or("INVALID");
        let expected = "GREASE-772-771|2-1.1|GREASE-4588-29-23-24|1027-2052-1025-1283-2053-1281-2054-1537|1|2|GREASE-4865-4866-4867-49195-49199-49196-49200-52393-52392-49171-49172-156-157-47-53|0-10-11-13-16-17613-18-23-27-35-43-45-5-51-65037-65281-GREASE-GREASE";

        println!("\n=== Peetprint hash ===");
        println!("Got:      {}", got);
        println!("Expected: {}", expected);

        // Assert equality
        assert_eq!(got, expected, "Peetprint mismatch!");
    }

    // Additional helpful fields (other keys in TLS)
    println!("\n=== TLS Info ===");
    if let Some(ja3) = tls.get("ja3") {
        println!("JA3: {}", ja3);
    }
    if let Some(ja3_ems) = tls.get("ja3_ems") {
        println!("JA3 EMS: {}", ja3_ems);
    }

    // Assertions
    assert!(http2.is_object(), "http2 field should be an object");

    // Verify HTTP/2 Fingerprint details
    let h2_obj = http2.as_object().unwrap();

    assert_eq!(
        h2_obj.get("akamai_fingerprint").unwrap().as_str().unwrap(),
        "1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p",
        "Akamai fingerprint mismatch"
    );

    assert_eq!(
        h2_obj
            .get("akamai_fingerprint_hash")
            .unwrap()
            .as_str()
            .unwrap(),
        "52d84b11737d980aef856699f885ca86",
        "Akamai hash mismatch"
    );

    let frames = h2_obj.get("sent_frames").unwrap().as_array().unwrap();

    // Check SETTINGS frame
    let settings_frame = &frames[0];
    assert_eq!(
        settings_frame.get("frame_type").unwrap().as_str().unwrap(),
        "SETTINGS"
    );

    let settings = settings_frame.get("settings").unwrap().as_array().unwrap();
    let settings_strs: Vec<&str> = settings.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(settings_strs.contains(&"HEADER_TABLE_SIZE = 65536"));
    assert!(settings_strs.contains(&"ENABLE_PUSH = 0"));
    assert!(settings_strs.contains(&"INITIAL_WINDOW_SIZE = 6291456"));
    assert!(settings_strs.contains(&"MAX_HEADER_LIST_SIZE = 262144"));

    // Check WINDOW_UPDATE frame
    let window_update_frame = &frames[1];
    assert_eq!(
        window_update_frame
            .get("frame_type")
            .unwrap()
            .as_str()
            .unwrap(),
        "WINDOW_UPDATE"
    );
    assert_eq!(
        window_update_frame
            .get("increment")
            .unwrap()
            .as_i64()
            .unwrap(),
        15663105
    );

    if !peetprint.is_null() {
        assert!(peetprint.is_string(), "peetprint should be a string");
        assert!(
            !peetprint.as_str().unwrap().is_empty(),
            "peetprint should not be empty"
        );
    }
}
