# Cronet-Cloak

**使用真实 Chrome TLS/HTTP2 指纹发送不可检测的 HTTP 请求**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org/)
[![Cronet](https://img.shields.io/badge/Cronet-141.0.7390.76-blue)](https://source.chromium.org/chromium/chromium/src/+/refs/tags/141.0.7390.76:components/cronet/)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

[English](README.md)

## 为什么选择 Cronet-Cloak？

现代反爬虫系统通过分析 **TLS 指纹**（JA3/JA4）和 **HTTP/2 指纹**（AKAMAI）来检测请求。像 `requests`、`curl` 或 `reqwest` 这样的库会暴露非浏览器指纹，从而触发机器人检测。

**Cronet-Cloak** 使用 Google 的 Cronet 库——与 Chrome 浏览器相同的网络栈——发送的请求与 **真实 Chrome 浏览器完全一致**。

### 指纹对比

| 库 | TLS 指纹 | HTTP/2 指纹 | 被检测风险 |
|---------|-----------------|-------------------|----------------|
| Python requests | ❌ 独特 | ❌ 无 | 🔴 高 |
| Node.js axios | ❌ 独特 | ❌ 无 | 🔴 高 |
| Go net/http | ❌ 独特 | ⚠️ 不同 | 🔴 高 |
| curl | ❌ 独特 | ⚠️ 不同 | 🟡 中 |
| **Cronet-Cloak** | ✅ Chrome | ✅ Chrome | 🟢 低 |

## 特性

- 🎭 **真实 Chrome 指纹** - 与 Chrome 浏览器完全相同的 TLS/JA3/JA4 和 HTTP/2 指纹
- 🚀 **高性能** - 原生 Chromium 网络栈，支持 QUIC、HTTP/2、Brotli
- 🔒 **代理支持** - HTTP、HTTPS、SOCKS5 代理及身份验证
- 📡 **REST API** - 简单的 JSON 接口，适配任何编程语言
- 🐳 **Docker 部署** - 一键启动

## 快速开始

### Docker 运行

```bash
docker run -p 3000:3000 ghcr.io/your-org/cronet-cloak
```

### 从源码编译

```bash
cargo build --release
cargo run
```

服务启动在 `http://0.0.0.0:3000`

## API 使用

### 发送请求

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

### 使用代理

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
      "username": "用户名",
      "password": "密码"
    }
  }
}
```

### 代理类型

| 类型 | 值 |
|------|-----|
| HTTP | 0 |
| HTTPS | 1 |
| SOCKS5 | 2 |

### 响应格式

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

> **注意：** 响应体为十六进制编码。

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                      你的应用程序                         │
└─────────────────────────┬───────────────────────────────┘
                          │ HTTP/JSON
┌─────────────────────────▼───────────────────────────────┐
│                    Cronet-Cloak                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │ Axum 服务器 │──│ FFI 封装层  │──│ Cronet 库       │  │
│  │  (Rust)     │  │   (Rust)    │  │ (Chrome C API)  │  │
│  └─────────────┘  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │ TLS 1.3 + HTTP/2
                          │ (Chrome 指纹)
┌─────────────────────────▼───────────────────────────────┐
│                       目标网站                           │
└─────────────────────────────────────────────────────────┘
```

## 应用场景

- 🛒 **电商监控** - 无阻断追踪价格
- 📊 **数据采集** - 爬取有激进反爬保护的网站
- 🔍 **安全研究** - 测试反爬虫系统
- 🌐 **API 访问** - 绕过基于指纹的速率限制

## 许可证

MIT
