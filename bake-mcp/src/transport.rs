use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Creates a safe stdio transport for the MCP server.
///
/// Features:
/// 1. Synchronized dedicated stdout writer task preventing interleaved chunks.
/// 2. Antigravity probe filter: cleanly answers `server/discover` probes with standard
///    JSON-RPC method not found errors (-32601) instead of crashing the rmcp deserializer.
/// 3. Isolates rmcp communication onto duplex streams.
pub async fn create_stdio_transport() -> (tokio::io::DuplexStream, tokio::io::DuplexStream) {
    let (server_in_write, server_in_read) = tokio::io::duplex(65536);
    let (server_out_read, server_out_write) = tokio::io::duplex(65536);
    let (stdout_tx, mut stdout_rx) = mpsc::channel::<Vec<u8>>(128);

    // Dedicated stdout writer task to guarantee synchronized, non-interleaving writes
    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        while let Some(chunk) = stdout_rx.recv().await {
            if let Err(e) = stdout.write_all(&chunk).await {
                tracing::debug!("Error writing to stdout: {:?}", e);
                break;
            }
            if let Err(e) = stdout.flush().await {
                tracing::debug!("Error flushing stdout: {:?}", e);
                break;
            }
        }
    });

    // Stdin reader & filter -> forwards to rmcp or responds to probes via stdout_tx
    let stdin_stdout_tx = stdout_tx.clone();
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        let mut server_in = server_in_write;

        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }

            // Check if this is an Antigravity server/discover probe
            if line.contains("server/discover")
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(&line)
                && v.get("method").and_then(|m| m.as_str()) == Some("server/discover")
            {
                let id = v.get("id").cloned().unwrap_or(serde_json::Value::Null);
                tracing::info!(
                    "Intercepted Antigravity server/discover probe with id {:?}",
                    id
                );
                let resp = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                });
                let resp_str = format!("{}\n", resp);
                if stdin_stdout_tx.send(resp_str.into_bytes()).await.is_err() {
                    break;
                }
                line.clear();
                continue;
            }

            // Forward clean line to rmcp
            if let Err(e) = server_in.write_all(line.as_bytes()).await {
                tracing::debug!("Error forwarding stdin to rmcp: {:?}", e);
                break;
            }
            let _ = server_in.flush().await;
            line.clear();
        }
    });

    // rmcp server output -> stdout_tx
    tokio::spawn(async move {
        let mut reader = BufReader::new(server_out_read);
        let mut buffer = [0u8; 8192];

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    if stdout_tx.send(buffer[..n].to_vec()).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::debug!("Error reading from rmcp server output: {:?}", e);
                    break;
                }
            }
        }
    });

    (server_in_read, server_out_write)
}

/// Schema transform: converts `"type": ["T", "null"]` arrays into
/// `"anyOf": [{"type": "T"}, {"type": "null"}]` branches on each property.
///
/// Many MCP clients only accept `type` as a single string and will reject or
/// silently drop the constraint when it's an array. Using `anyOf` is the
/// portable way to express nullable types in JSON Schema 2020-12.
#[allow(dead_code)]
pub fn nullable_to_anyof(schema: &mut schemars::Schema) {
    if let Some(obj) = schema.as_object_mut()
        && let Some(props) = obj.get_mut("properties")
        && let Some(props_map) = props.as_object_mut()
    {
        for (_key, prop_schema) in props_map.iter_mut() {
            if let Some(prop_obj) = prop_schema.as_object_mut()
                && let Some(type_val) = prop_obj.remove("type")
            {
                if let Some(type_arr) = type_val.as_array() {
                    let mut branches: Vec<serde_json::Value> = Vec::new();
                    for t in type_arr {
                        if t.as_str() == Some("null") {
                            branches.push(serde_json::json!({"type": "null"}));
                        } else {
                            let mut branch = prop_obj.clone();
                            branch.insert("type".to_string(), t.clone());
                            branch.remove("anyOf");
                            branches.push(serde_json::Value::Object(branch));
                        }
                    }
                    prop_obj.remove("format");
                    prop_obj.remove("minimum");
                    prop_obj.remove("maximum");
                    prop_obj.insert("anyOf".to_string(), serde_json::Value::Array(branches));
                } else {
                    prop_obj.insert("type".to_string(), type_val);
                }
            }
        }
    }
}
