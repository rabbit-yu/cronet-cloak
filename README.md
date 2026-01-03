# Cronet-Cloak

**Undetectable HTTP requests with authentic Chrome TLS/HTTP2 fingerprints**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org/)
[![Cronet](https://img.shields.io/badge/Cronet-141.0.7390.76-blue)](https://source.chromium.org/chromium/chromium/src/+/refs/tags/141.0.7390.76:components/cronet/)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

[中文文档](README_CN.md)

## Why Cronet-Cloak?

Modern anti-bot systems detect requests by analyzing **TLS fingerprints** (JA3/JA4) and **HTTP/2 fingerprints** (AKAMAI). Libraries like `requests`, `curl`, or even `reqwest` expose non-browser fingerprints that trigger bot detection.

**Cronet-Cloak** uses Google's Cronet library — the same networking stack that powers Chrome — to make requests that are **indistinguishable from a real Chrome browser**.

### Fingerprint Comparison

| Library | TLS Fingerprint | HTTP/2 Fingerprint | Detection Risk |
|---------|-----------------|-------------------|----------------|
| Python requests | ❌ Unique | ❌ None | 🔴 High |
| Node.js axios | ❌ Unique | ❌ None | 🔴 High |
| Go net/http | ❌ Unique | ⚠️ Different | 🔴 High |
| curl | ❌ Unique | ⚠️ Different | 🟡 Medium |
| **Cronet-Cloak** | ✅ Chrome | ✅ Chrome | 🟢 Low |

## Features

- 🎭 **Real Chrome Fingerprint** - Identical TLS/JA3/JA4 and HTTP/2 fingerprints as Chrome browser
- 🚀 **High Performance** - Native Chromium networking stack with QUIC, HTTP/2, Brotli
- 🔒 **Proxy Support** - HTTP, HTTPS, SOCKS5 with authentication
- 📡 **REST API** - Simple JSON interface for any language
- 🐳 **Docker Ready** - One-command deployment

## Quick Start

### Run with Docker

```bash
docker run -p 3000:3000 ghcr.io/your-org/cronet-cloak
```

### Build from Source

```bash
cargo build --release
cargo run
```

Server starts at `http://0.0.0.0:3000`

## API Usage

### Make a Request

```bash
curl -X POST http://localhost:3000/api/v1/execute \
  -H "Content-Type: application/json" \
  -d '{
    "request_id": "test-1",
    "target": {
      "url": "https://tls.peet.ws/api/all",
      "method": "GET"
    },
    "config": {
      "follow_redirects": true
    }
  }'
```

### With Proxy

```json
{
  "request_id": "proxy-test",
  "target": {
    "url": "https://example.com",
    "method": "GET"
  },
  "config": {
    "proxy": {
      "type": 0,
      "host": "proxy.example.com",
      "port": 8080,
      "username": "user",
      "password": "pass"
    }
  }
}
```

### Proxy Types

| Type | Value |
|------|-------|
| HTTP | 0 |
| HTTPS | 1 |
| SOCKS5 | 2 |

### Response Format

```json
{
  "request_id": "test-1",
  "success": true,
  "response": {
    "status_code": 200,
    "body": "7b22..."
  },
  "duration_ms": 150
}
```

> **Note:** Response body is hex-encoded.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Your Application                     │
└─────────────────────────┬───────────────────────────────┘
                          │ HTTP/JSON
┌─────────────────────────▼───────────────────────────────┐
│                    Cronet-Cloak                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Axum Server │──│ FFI Wrapper │──│ Cronet Library  │  │
│  │  (Rust)     │  │   (Rust)    │  │ (Chrome C API)  │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │ TLS 1.3 + HTTP/2
                          │ (Chrome Fingerprint)
┌─────────────────────────▼───────────────────────────────┐
│                    Target Website                        │
└─────────────────────────────────────────────────────────┘
```

## Use Cases

- 🛒 **E-commerce monitoring** - Track prices without blocks
- 📊 **Data collection** - Scrape sites with aggressive bot protection
- 🔍 **Security research** - Test anti-bot systems
- 🌐 **API access** - Bypass fingerprint-based rate limits

## License

MIT
