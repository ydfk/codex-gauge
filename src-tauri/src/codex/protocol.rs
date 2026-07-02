use serde_json::{json, Value};

pub fn initialize_request(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "codex-gauge",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {},
            "protocolVersion": "2024-11-05"
        }
    })
}

pub fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    })
}

pub fn method_request(id: u64, method: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method
    })
}
