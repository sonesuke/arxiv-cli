use crate::core::{ArxivClient, Config};
use async_trait::async_trait;
use mcp_sdk_rs::server::{Server, ServerHandler};
use mcp_sdk_rs::transport::stdio::StdioTransport;
use mcp_sdk_rs::types::{
    ClientCapabilities, Implementation, MessageContent, ServerCapabilities, Tool, ToolResult,
    ToolSchema,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

pub struct ArxivHandler {
    client: ArxivClient,
}

#[async_trait]
impl ServerHandler for ArxivHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, mcp_sdk_rs::error::Error> {
        Ok(ServerCapabilities {
            tools: Some(json!({
                "listChanged": false
            })),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<(), mcp_sdk_rs::error::Error> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, mcp_sdk_rs::error::Error> {
        match method {
            "tools/list" => {
                let tools = vec![
                    Tool {
                        name: "search_papers".to_string(),
                        description: "Search arXiv for papers matching a query".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "query": {
                                    "type": "string",
                                    "description": "The search query (e.g., 'quantum computing')"
                                },
                                "limit": {
                                    "type": "number",
                                    "description": "Maximum number of results to return"
                                }
                            })),
                            required: Some(vec!["query".to_string()]),
                        }),
                        annotations: None,
                    },
                    Tool {
                        name: "fetch_paper".to_string(),
                        description: "Fetch details of a specific paper by ID".to_string(),
                        input_schema: Some(ToolSchema {
                            properties: Some(json!({
                                "id": {
                                    "type": "string",
                                    "description": "The arXiv ID of the paper (e.g., '2512.04518')"
                                }
                            })),
                            required: Some(vec!["id".to_string()]),
                        }),
                        annotations: None,
                    },
                ];
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let params = params.ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing parameters",
                    )
                })?;
                let name = params["name"].as_str().ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing tool name",
                    )
                })?;
                let arguments = params["arguments"].as_object().ok_or_else(|| {
                    mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::InvalidParams,
                        "Missing arguments",
                    )
                })?;

                match name {
                    "search_papers" => {
                        let query =
                            arguments.get("query").and_then(|v| v.as_str()).unwrap_or_default();
                        let limit =
                            arguments.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

                        let result = match self.client.search(query, limit, None, None).await {
                            Ok(papers) => ToolResult {
                                content: vec![MessageContent::Text {
                                    text: serde_json::to_string_pretty(&papers).unwrap_or_default(),
                                }],
                                structured_content: None,
                            },
                            Err(e) => ToolResult {
                                content: vec![MessageContent::Text {
                                    text: format!("Search failed: {}", e),
                                }],
                                structured_content: None,
                            },
                        };
                        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
                    }
                    "fetch_paper" => {
                        let id = arguments.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                        let result = match self.client.fetch(id).await {
                            Ok(paper) => ToolResult {
                                content: vec![MessageContent::Text {
                                    text: serde_json::to_string_pretty(&paper).unwrap_or_default(),
                                }],
                                structured_content: None,
                            },
                            Err(e) => ToolResult {
                                content: vec![MessageContent::Text {
                                    text: format!("Fetch failed: {}", e),
                                }],
                                structured_content: None,
                            },
                        };
                        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
                    }
                    _ => Err(mcp_sdk_rs::error::Error::protocol(
                        mcp_sdk_rs::error::ErrorCode::MethodNotFound,
                        format!("Unknown tool: {}", name),
                    )),
                }
            }
            _ => Err(mcp_sdk_rs::error::Error::protocol(
                mcp_sdk_rs::error::ErrorCode::MethodNotFound,
                format!("Unknown method: {}", method),
            )),
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    let config = Config::load().map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
    let client =
        ArxivClient::new(&config).await.map_err(|e| anyhow::anyhow!("Client error: {}", e))?;
    let handler = Arc::new(ArxivHandler { client });

    let (read_tx, read_rx) = mpsc::channel::<String>(32);
    let (write_tx, mut write_rx) = mpsc::channel::<String>(32);

    // Thread for reading from stdin
    tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = read_tx.send(line).await;
        }
    });

    // Thread for writing to stdout
    tokio::spawn(async move {
        let mut stdout = io::stdout();
        while let Some(line) = write_rx.recv().await {
            let _ = stdout.write_all(line.as_bytes()).await;
            let _ = stdout.write_all(b"\n").await;
            let _ = stdout.flush().await;
        }
    });

    let transport = Arc::new(StdioTransport::new(read_rx, write_tx));

    let server = Server::new(transport, handler);
    server.start().await.map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}
