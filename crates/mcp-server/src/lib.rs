pub mod protocol;
pub mod tools;

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use tools::{
    list_mcp_tools, tool_create_deck, tool_inspect_slide, tool_install_package, tool_package_deck,
    tool_upsert_slide, validate_package_name, validate_safe_slide_filename,
    validate_slide_source_code, DeckManifestFile, SlideManifestEntry,
};

use foil_packager::PackagerError;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("packager error: {0}")]
    Packager(#[from] PackagerError),
    #[error("security violation: {0}")]
    SecurityViolation(String),
    #[error("tool execution failed: {0}")]
    ToolExecutionFailed(String),
}

pub fn run_mcp_stdio_server(workspace_dir: PathBuf) -> Result<(), McpError> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => {
                if req.id.is_none() {
                    continue;
                }
                handle_rpc_request(&workspace_dir, req)
            }
            Err(err) => JsonRpcResponse::error(Value::Null, -32700, format!("parse error: {err}")),
        };
        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        stdout.write_all(&encoded)?;
        stdout.flush()?;
    }

    Ok(())
}

pub fn handle_rpc_request(workspace_dir: &Path, request: JsonRpcRequest) -> JsonRpcResponse {
    let id = request.id.unwrap_or(Value::Null);

    match request.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "foil-mcp-server", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "tools/list" => JsonRpcResponse::success(id, list_mcp_tools()),
        "tools/call" => {
            let Some(params) = request.params else {
                return JsonRpcResponse::error(id, -32602, "missing params");
            };
            let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
                return JsonRpcResponse::error(id, -32602, "missing tool name");
            };
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            match dispatch_tool_call(workspace_dir, tool_name, &args) {
                Ok(payload) => JsonRpcResponse::success(
                    id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string(&payload).unwrap_or_default()
                            }
                        ]
                    }),
                ),
                Err(err) => JsonRpcResponse::error(id, -32000, err.to_string()),
            }
        }
        other => JsonRpcResponse::error(id, -32601, format!("method not found: {other}")),
    }
}

fn dispatch_tool_call(
    workspace_dir: &Path,
    tool_name: &str,
    args: &Value,
) -> Result<Value, McpError> {
    match tool_name {
        "create_deck" => {
            let title = args
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::ToolExecutionFailed("missing title".to_string()))?;
            let theme = args.get("theme").and_then(Value::as_str);
            let aspect_ratio = args.get("aspect_ratio").and_then(Value::as_str);
            let manifest = tool_create_deck(workspace_dir, title, theme, aspect_ratio)?;
            Ok(serde_json::to_value(manifest)?)
        }
        "upsert_slide" => {
            let index = args
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let filename = args
                .get("filename")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::ToolExecutionFailed("missing filename".to_string()))?;
            let code = args
                .get("code")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::ToolExecutionFailed("missing code".to_string()))?;
            let speaker_notes = args
                .get("speaker_notes")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let entry = tool_upsert_slide(workspace_dir, index, filename, code, speaker_notes)?;
            Ok(serde_json::to_value(entry)?)
        }
        "install_package" => {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| McpError::ToolExecutionFailed("missing package name".to_string()))?;
            let msg = tool_install_package(workspace_dir, name)?;
            Ok(json!({ "status": msg }))
        }
        "inspect_slide" => {
            let slide_index = args
                .get("slideIndex")
                .or_else(|| args.get("slide_index"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let step_index = args
                .get("stepIndex")
                .or_else(|| args.get("step_index"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let bridge_port = args
                .get("bridgePort")
                .and_then(Value::as_u64)
                .map(|p| p as u16);
            let res = tool_inspect_slide(slide_index, step_index, bridge_port)?;
            Ok(serde_json::to_value(res)?)
        }
        "package_deck" => {
            let output_path = args
                .get("output_path")
                .and_then(Value::as_str)
                .unwrap_or("deck.foil");
            let report = tool_package_deck(workspace_dir, output_path)?;
            Ok(serde_json::to_value(report)?)
        }
        other => Err(McpError::ToolExecutionFailed(format!(
            "unknown tool: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mcp_create_deck_and_upsert_slide_and_package_deck_flow() {
        let dir = tempdir().expect("tempdir");

        let create_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "create_deck",
                "arguments": {
                    "title": "Q4 Pitch",
                    "theme": "default",
                    "aspect_ratio": "16:9"
                }
            })),
        };
        let create_resp = handle_rpc_request(dir.path(), create_req);
        assert!(create_resp.error.is_none());

        let upsert_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "upsert_slide",
                "arguments": {
                    "index": 0,
                    "filename": "01-intro.js",
                    "code": "export default { html: '<section class=\"slide\"><h1>Hello</h1></section>' };",
                    "speaker_notes": "Welcome everyone"
                }
            })),
        };
        let upsert_resp = handle_rpc_request(dir.path(), upsert_req);
        assert!(upsert_resp.error.is_none());
        assert!(dir.path().join("src/slides/01-intro.js").is_file());

        let pkg_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "package_deck",
                "arguments": {
                    "output_path": "pitch.foil"
                }
            })),
        };
        let pkg_resp = handle_rpc_request(dir.path(), pkg_req);
        assert!(pkg_resp.error.is_none());
        assert!(dir.path().join("pitch.foil").is_file());
    }

    #[test]
    fn test_mcp_upsert_slide_blocks_path_traversal_and_malicious_code() {
        let dir = tempdir().expect("tempdir");

        let traversal_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(10)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "upsert_slide",
                "arguments": {
                    "index": 0,
                    "filename": "../../evil.js",
                    "code": "export default { html: '<div>ok</div>' };"
                }
            })),
        };
        let traversal_resp = handle_rpc_request(dir.path(), traversal_req);
        assert!(traversal_resp.error.is_some());

        let eval_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(11)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "upsert_slide",
                "arguments": {
                    "index": 0,
                    "filename": "01-evil.js",
                    "code": "export default { onEnter() { eval('hack()'); } };"
                }
            })),
        };
        let eval_resp = handle_rpc_request(dir.path(), eval_req);
        assert!(eval_resp.error.is_some());
    }

    #[test]
    fn test_mcp_install_package_rejects_shell_injection_names() {
        assert!(validate_package_name("motion").is_ok());
        assert!(validate_package_name("@foil-engine/runtime").is_ok());
        assert!(validate_package_name("three; rm -rf /").is_err());
        assert!(validate_package_name("--evil-flag").is_err());
    }
}
