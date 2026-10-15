use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<JsonRpcId>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<JsonRpcId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

pub mod tools;

pub fn start_server(db_name: &std::path::PathBuf, assets: &std::path::PathBuf) -> Result<()> {
    crate::watcher::spawn_indexing_thread(db_name.clone(), assets.clone());

    let device = witchcraft::make_device();
    let embedder = witchcraft::Embedder::new(&device, assets)?;
    let mut cache = witchcraft::EmbeddingsCache::new(1);
    let db = witchcraft::DB::new_reader(db_name.clone())?;

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // EOF
        }

        if let Some(resp_json) = handle_request(&line, &db, &embedder, &mut cache) {
            println!("{}", resp_json);
        }
    }
    Ok(())
}

pub fn handle_request(
    raw_json: &str,
    db: &witchcraft::DB,
    embedder: &witchcraft::Embedder,
    cache: &mut witchcraft::EmbeddingsCache,
) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(raw_json) {
        Ok(r) => r,
        Err(_) => return None,
    };
    
    if req.method == "initialize" {
        return Some(serde_json::to_string(&JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "pickbrain",
                    "version": "0.1.0"
                }
            })),
            error: None,
        }).unwrap());
    } else if req.method == "tools/list" {
        return Some(serde_json::to_string(&JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(tools::list_tools()),
            error: None,
        }).unwrap());
    } else if req.method == "tools/call" {
        if let Some(params) = req.params {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned();
            
            if let Ok(result) = tools::handle_tool_call(name, args, db, embedder, cache) {
                return Some(serde_json::to_string(&JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: Some(result),
                    error: None,
                }).unwrap());
            } else {
                return Some(serde_json::to_string(&JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: req.id,
                    result: None,
                    error: Some(serde_json::json!({ "code": -32603, "message": "Tool execution failed" })),
                }).unwrap());
            }
        }
    }
    None
}

// In unit tests we omit state execution by ignoring test outputs
// We only run mcp struct tests on a mock integration suite within core tests.
