use crate::capture;
use crate::cli::{CoordMode, ImageFormat, Region};
use crate::doctor;
use crate::output::{self, OutputDestination};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::str::FromStr;

const BASE64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Fast, dependency-free standard base64 encoding (RFC 4648).
pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        out.push(BASE64_CHARS[(b0 >> 2) as usize] as char);
        out.push(BASE64_CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if i + 1 < input.len() {
            out.push(BASE64_CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < input.len() {
            out.push(BASE64_CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }
    out
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: &'static str,
    pub id: serde_json::Value,
    pub result: T,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: &'static str,
    pub id: serde_json::Value,
    pub error: JsonRpcError,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ToolListResult {
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// Handle a single incoming JSON-RPC line and return serialized response line if applicable.
pub fn handle_mcp_message(line: &str) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            debug!("Failed to parse JSON-RPC request: {e}");
            let err = JsonRpcErrorResponse {
                jsonrpc: "2.0",
                id: serde_json::Value::Null,
                error: JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {e}"),
                    data: None,
                },
            };
            return serde_json::to_string(&err).ok();
        }
    };

    let id = req.id?;

    if req.jsonrpc != "2.0" {
        let err = JsonRpcErrorResponse {
            jsonrpc: "2.0",
            id,
            error: JsonRpcError {
                code: -32600,
                message: format!(
                    "Invalid Request: jsonrpc must be '2.0', got '{}'",
                    req.jsonrpc
                ),
                data: None,
            },
        };
        return serde_json::to_string(&err).ok();
    }

    match req.method.as_str() {
        "initialize" => {
            let res = JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: InitializeResult {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: ServerCapabilities {
                        tools: serde_json::json!({}),
                    },
                    server_info: ServerInfo {
                        name: "rgc-cli".to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                },
            };
            serde_json::to_string(&res).ok()
        }
        "ping" => {
            let res = JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: serde_json::json!({}),
            };
            serde_json::to_string(&res).ok()
        }
        "tools/list" => {
            let tools = vec![
                ToolDefinition {
                    name: "list_monitors".to_string(),
                    description: "List all active physical displays/monitors with index, dimensions, DPI scale factor, and primary display flag.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                },
                ToolDefinition {
                    name: "capture_screen".to_string(),
                    description: "Capture screen image from a monitor or rectangular region. Can return image base64 data directly or save to a file path.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "monitor": {
                                "type": "integer",
                                "description": "0-indexed monitor index. If omitted, captures primary display."
                            },
                            "region": {
                                "type": "string",
                                "description": "Cropping region in 'x,y,w,h' format (e.g. '100,100,800,600')."
                            },
                            "coords": {
                                "type": "string",
                                "enum": ["logical", "physical"],
                                "description": "Coordinate system for cropping region: 'logical' (scaled by display DPI) or 'physical' (raw pixels). Defaults to 'logical'."
                            },
                            "format": {
                                "type": "string",
                                "enum": ["png", "jpeg", "webp"],
                                "description": "Image format ('png', 'jpeg', or 'webp'). Defaults to 'png'."
                            },
                            "scale": {
                                "type": "number",
                                "description": "Optional downscale factor between 0.01 and 1.0 (e.g. 0.5 for half resolution to reduce bandwidth/tokens)."
                            },
                            "maxDimension": {
                                "type": "integer",
                                "description": "Optional maximum width or height in pixels (preserves aspect ratio)."
                            },
                            "quality": {
                                "type": "integer",
                                "description": "Optional JPEG compression quality (1-100, defaults to 85)."
                            },
                            "outputPath": {
                                "type": "string",
                                "description": "Optional file path to write image to disk. If omitted, image is returned as base64."
                            },
                            "returnBase64": {
                                "type": "boolean",
                                "description": "Whether to return image base64 data in the response. Defaults to true if outputPath is omitted."
                            }
                        },
                        "additionalProperties": false
                    }),
                },
                ToolDefinition {
                    name: "doctor".to_string(),
                    description: "Run hardware, display, and environment diagnostics to check screen capture availability.".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                },
            ];

            let res = JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: ToolListResult { tools },
            };
            serde_json::to_string(&res).ok()
        }
        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            let call_result = execute_tool_call(tool_name, &arguments);

            let res = JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: call_result,
            };
            serde_json::to_string(&res).ok()
        }
        _ => {
            let err = JsonRpcErrorResponse {
                jsonrpc: "2.0",
                id,
                error: JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                    data: None,
                },
            };
            serde_json::to_string(&err).ok()
        }
    }
}

fn execute_tool_call(name: &str, arguments: &serde_json::Value) -> ToolCallResult {
    match name {
        "list_monitors" => match capture::get_monitor_infos() {
            Ok(monitors) => match serde_json::to_string_pretty(&monitors) {
                Ok(text) => ToolCallResult {
                    content: vec![ToolContent::Text { text }],
                    is_error: false,
                },
                Err(e) => ToolCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Serialization failed: {e}"),
                    }],
                    is_error: true,
                },
            },
            Err(e) => ToolCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Failed to list monitors: {e}"),
                }],
                is_error: true,
            },
        },
        "doctor" => {
            let report = doctor::run_doctor();
            match serde_json::to_string_pretty(&report) {
                Ok(text) => ToolCallResult {
                    content: vec![ToolContent::Text { text }],
                    is_error: false,
                },
                Err(e) => ToolCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Serialization failed: {e}"),
                    }],
                    is_error: true,
                },
            }
        }
        "capture_screen" => execute_capture_tool(arguments),
        _ => ToolCallResult {
            content: vec![ToolContent::Text {
                text: format!("Unknown tool name: '{name}'"),
            }],
            is_error: true,
        },
    }
}

fn execute_capture_tool(arguments: &serde_json::Value) -> ToolCallResult {
    let monitor = if let Some(m_val) = arguments.get("monitor") {
        if let Some(idx) = m_val.as_u64() {
            Some(idx as usize)
        } else if let Some(s) = m_val.as_str() {
            match s.trim().parse::<usize>() {
                Ok(idx) => Some(idx),
                Err(_) => {
                    return ToolCallResult {
                        content: vec![ToolContent::Text {
                            text: format!(
                                "Invalid monitor index '{s}': expected a non-negative integer"
                            ),
                        }],
                        is_error: true,
                    };
                }
            }
        } else if !m_val.is_null() {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Invalid monitor argument '{m_val}': expected integer index"),
                }],
                is_error: true,
            };
        } else {
            None
        }
    } else {
        None
    };

    let region = if let Some(reg_val) = arguments.get("region") {
        if let Some(reg_str) = reg_val.as_str() {
            match Region::from_str(reg_str) {
                Ok(r) => Some(r),
                Err(e) => {
                    return ToolCallResult {
                        content: vec![ToolContent::Text {
                            text: format!("Invalid region '{reg_str}': {e}"),
                        }],
                        is_error: true,
                    };
                }
            }
        } else if !reg_val.is_null() {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: format!(
                        "Invalid region parameter '{reg_val}': expected a string in 'x,y,w,h' format (e.g. '100,100,800,600')"
                    ),
                }],
                is_error: true,
            };
        } else {
            None
        }
    } else {
        None
    };

    let coords = match arguments.get("coords").and_then(|v| v.as_str()) {
        Some("physical") => CoordMode::Physical,
        _ => CoordMode::Logical,
    };

    let format = match arguments.get("format").and_then(|v| v.as_str()) {
        Some("jpeg") | Some("jpg") => ImageFormat::Jpeg,
        Some("webp") => ImageFormat::Webp,
        _ => ImageFormat::Png,
    };

    let scale = arguments.get("scale").and_then(|v| v.as_f64());
    if let Some(s) = scale {
        if !s.is_finite() || s <= 0.0 || s > 1.0 {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: format!(
                        "Invalid scale '{s}': expected a positive number between 0.01 and 1.0"
                    ),
                }],
                is_error: true,
            };
        }
    }

    let max_dimension = arguments
        .get("maxDimension")
        .and_then(|v| v.as_u64())
        .map(|d| d as u32);
    if let Some(d) = max_dimension {
        if d == 0 {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: "Invalid maxDimension: must be greater than 0".to_string(),
                }],
                is_error: true,
            };
        }
    }

    let quality = arguments
        .get("quality")
        .and_then(|v| v.as_u64())
        .map(|q| q as u8);
    if let Some(q) = quality {
        if q == 0 || q > 100 {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: "Invalid quality: must be between 1 and 100".to_string(),
                }],
                is_error: true,
            };
        }
    }

    let output_path = arguments
        .get("outputPath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let return_base64 = arguments
        .get("returnBase64")
        .and_then(|v| v.as_bool())
        .unwrap_or(output_path.is_none());

    if output_path.is_none() && !return_base64 {
        return ToolCallResult {
            content: vec![ToolContent::Text {
                text: "Invalid capture request: either 'outputPath' must be provided or 'returnBase64' must be true.".to_string(),
            }],
            is_error: true,
        };
    }

    let image = match capture::capture_screen(monitor, region.as_ref(), coords) {
        Ok(img) => img,
        Err(e) => {
            return ToolCallResult {
                content: vec![ToolContent::Text {
                    text: format!("Capture failed: {e}"),
                }],
                is_error: true,
            };
        }
    };

    let image = output::apply_image_transforms(image, scale, max_dimension);

    let mut content = Vec::new();

    if let Some(ref path) = output_path {
        let destination = OutputDestination::File(path.clone());
        let saved_path = match output::save_image(&image, &destination, format, quality) {
            Ok(p) => p,
            Err(e) => {
                return ToolCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to save image to '{}': {e}", path.display()),
                    }],
                    is_error: true,
                };
            }
        };
        content.push(ToolContent::Text {
            text: format!("Image successfully saved to: {}", saved_path.display()),
        });
    }

    if return_base64 {
        let bytes = match output::encode_image_bytes(&image, format, quality) {
            Ok(b) => b,
            Err(e) => {
                return ToolCallResult {
                    content: vec![ToolContent::Text {
                        text: format!("Failed to encode image as {format:?}: {e}"),
                    }],
                    is_error: true,
                };
            }
        };

        let b64 = base64_encode(&bytes);
        let mime_type = match format {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Webp => "image/webp",
        };

        content.push(ToolContent::Image {
            data: b64,
            mime_type: mime_type.to_string(),
        });
        content.push(ToolContent::Text {
            text: format!(
                "Captured {}x{} image ({:?}, {} bytes)",
                image.width(),
                image.height(),
                format,
                bytes.len()
            ),
        });
    }

    ToolCallResult {
        content,
        is_error: false,
    }
}

/// Run Model Context Protocol (MCP) server loop over standard I/O streams.
pub fn run_mcp_loop(reader: impl BufRead, mut writer: impl Write) -> std::io::Result<()> {
    info!("Starting MCP stdio server loop");
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::BrokenPipe
                {
                    break;
                }
                error!("Error reading line from MCP stdin: {e}");
                return Err(e);
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Some(response_json) = handle_mcp_message(&line) {
            if let Err(e) = writeln!(writer, "{response_json}") {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    debug!("Downstream MCP client closed pipe (BrokenPipe)");
                    break;
                }
                return Err(e);
            }
            if let Err(e) = writer.flush() {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    debug!("Downstream MCP client closed pipe during flush (BrokenPipe)");
                    break;
                }
                return Err(e);
            }
        }
    }
    info!("Exiting MCP stdio server loop");
    Ok(())
}

/// Run standard Model Context Protocol (MCP) server over stdin and stdout.
pub fn run_mcp_server() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_mcp_loop(stdin.lock(), stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_rfc_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_handle_mcp_initialize() {
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""protocolVersion":"2024-11-05""#));
        assert!(resp.contains(r#""name":"rgc-cli""#));
    }

    #[test]
    fn test_handle_mcp_ping() {
        let req = r#"{"jsonrpc":"2.0","id":"ping-1","method":"ping"}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""id":"ping-1""#));
        assert!(resp.contains(r#""result":{}"#));
    }

    #[test]
    fn test_handle_mcp_tools_list() {
        let req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains("list_monitors"));
        assert!(resp.contains("capture_screen"));
        assert!(resp.contains("doctor"));
    }

    #[test]
    fn test_handle_mcp_tool_call_doctor() {
        let req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"doctor","arguments":{}}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""isError":false"#));
        assert!(
            resp.contains("Operating System")
                || resp.contains("supportedFormats")
                || resp.contains("backend")
        );
    }

    #[test]
    fn test_handle_mcp_tool_call_unknown() {
        let req = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"non_existent_tool","arguments":{}}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""isError":true"#));
        assert!(resp.contains("Unknown tool name"));
    }

    #[test]
    fn test_handle_mcp_method_not_found() {
        let req = r#"{"jsonrpc":"2.0","id":5,"method":"unknown_method"}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""code":-32601"#));
        assert!(resp.contains("Method not found"));
    }

    #[test]
    fn test_handle_mcp_invalid_version() {
        let req = r#"{"jsonrpc":"1.0","id":6,"method":"ping"}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""code":-32600"#));
        assert!(resp.contains("jsonrpc must be '2.0'"));
    }

    #[test]
    fn test_handle_mcp_tool_call_invalid_region() {
        let req = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"capture_screen","arguments":{"region":"invalid"}}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""isError":true"#));
        assert!(resp.contains("Invalid region"));

        let req_obj = r#"{"jsonrpc":"2.0","id":71,"method":"tools/call","params":{"name":"capture_screen","arguments":{"region":{"x":0,"y":0}}}}"#;
        let resp_obj = handle_mcp_message(req_obj).expect("response expected");
        assert!(resp_obj.contains(r#""isError":true"#));
        assert!(resp_obj.contains("Invalid region parameter"));
    }

    #[test]
    fn test_handle_mcp_tool_call_invalid_monitor() {
        let req = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"capture_screen","arguments":{"monitor":99999}}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""isError":true"#));
        assert!(resp.contains("Capture failed"));

        // Monitor as numeric string is parsed
        let req_str = r#"{"jsonrpc":"2.0","id":81,"method":"tools/call","params":{"name":"capture_screen","arguments":{"monitor":"99999"}}}"#;
        let resp_str = handle_mcp_message(req_str).expect("response expected");
        assert!(resp_str.contains(r#""isError":true"#));
        assert!(resp_str.contains("Capture failed"));

        // Non-numeric monitor string returns validation error
        let req_bad_str = r#"{"jsonrpc":"2.0","id":82,"method":"tools/call","params":{"name":"capture_screen","arguments":{"monitor":"abc"}}}"#;
        let resp_bad_str = handle_mcp_message(req_bad_str).expect("response expected");
        assert!(resp_bad_str.contains(r#""isError":true"#));
        assert!(resp_bad_str.contains("expected a non-negative integer"));
    }

    #[test]
    fn test_handle_mcp_tool_call_no_output_or_base64() {
        let req = r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"capture_screen","arguments":{"returnBase64":false}}}"#;
        let resp = handle_mcp_message(req).expect("response expected");
        assert!(resp.contains(r#""isError":true"#));
        assert!(
            resp.contains("either 'outputPath' must be provided or 'returnBase64' must be true")
        );
    }

    #[test]
    fn test_handle_mcp_tool_call_invalid_scale_quality() {
        let req_scale = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"capture_screen","arguments":{"scale":1.5}}}"#;
        let resp_scale = handle_mcp_message(req_scale).expect("response expected");
        assert!(resp_scale.contains(r#""isError":true"#));
        assert!(resp_scale.contains("Invalid scale"));

        let req_dim = r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"capture_screen","arguments":{"maxDimension":0}}}"#;
        let resp_dim = handle_mcp_message(req_dim).expect("response expected");
        assert!(resp_dim.contains(r#""isError":true"#));
        assert!(resp_dim.contains("Invalid maxDimension"));

        let req_quality = r#"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"capture_screen","arguments":{"quality":105}}}"#;
        let resp_quality = handle_mcp_message(req_quality).expect("response expected");
        assert!(resp_quality.contains(r#""isError":true"#));
        assert!(resp_quality.contains("Invalid quality"));
    }

    #[test]
    fn test_mcp_loop_execution() {
        let input = "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"ping\"}\n";
        let mut output = Vec::new();
        run_mcp_loop(input.as_bytes(), &mut output).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains(r#""id":10"#));
    }
}
