use crate::core::{ArxivClient, Config};
use cypher_rs::CypherEngine;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars::JsonSchema,
    service::NotificationContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tokio::sync::RwLock;

// Tool request parameter structures

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SearchPapersRequest {
    #[schemars(description = "The search query (e.g., 'quantum computing')")]
    pub query: String,

    #[schemars(description = "Output file path to write results (JSON format)")]
    pub output_path: String,

    #[schemars(description = "Maximum number of results to return")]
    #[serde(default)]
    pub limit: Option<usize>,

    #[schemars(description = "Filter by date (submitted before), format: YYYY-MM-DD")]
    #[serde(default)]
    pub before: Option<String>,

    #[schemars(description = "Filter by date (submitted after), format: YYYY-MM-DD")]
    #[serde(default)]
    pub after: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct FetchPaperRequest {
    #[schemars(description = "The arXiv ID of the paper (e.g., '2512.04518')")]
    pub id: String,

    #[schemars(description = "Output file path to write results (JSON format)")]
    pub output_path: String,

    #[schemars(
        description = "If true, downloads the raw PDF to a local temporary file and returns its path"
    )]
    #[serde(default)]
    pub raw: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ExecuteCypherRequest {
    #[schemars(description = "Cypher query to execute")]
    pub query: String,
}

pub struct ArxivHandler {
    client: ArxivClient,
    tool_router: ToolRouter<ArxivHandler>,
    query_engine: Arc<RwLock<Option<CypherEngine>>>,
}

#[tool_router(router = tool_router)]
impl ArxivHandler {
    pub fn new(client: ArxivClient) -> Self {
        Self { client, tool_router: Self::tool_router(), query_engine: Arc::new(RwLock::new(None)) }
    }

    #[tool(description = "Search arXiv for papers matching a query")]
    pub async fn search_papers(
        &self,
        Parameters(request): Parameters<SearchPapersRequest>,
    ) -> Result<String, ErrorData> {
        let SearchPapersRequest { query, limit, before, after, output_path } = request;

        let papers =
            self.client.search(&query, limit, after, before, false).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to search arXiv: {}", e), None)
            })?;

        // Create CypherEngine with auto-detection
        let json_value: serde_json::Value = serde_json::to_value(&papers).map_err(|e| {
            ErrorData::internal_error(format!("Failed to serialize papers: {}", e), None)
        })?;

        let engine = CypherEngine::from_json_auto(&json_value).map_err(|e| {
            ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
        })?;

        // Get graph schema from CypherEngine
        let graph_schema = engine.get_schema();

        // Store in handler state
        *self.query_engine.write().await = Some(engine);

        // Write to file
        let json = serde_json::to_string_pretty(&papers).map_err(|e| {
            ErrorData::internal_error(format!("Failed to serialize papers: {}", e), None)
        })?;
        tokio::fs::write(&output_path, json).await.map_err(|e| {
            ErrorData::internal_error(format!("Failed to write to file: {}", e), None)
        })?;

        let result = serde_json::json!({
            "output_path": output_path,
            "count": papers.len(),
            "graph_schema": graph_schema
        });

        serde_json::to_string_pretty(&result).map_err(|e| {
            ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
        })
    }

    #[tool(description = "Fetch details of a specific paper by ID")]
    pub async fn fetch_paper(
        &self,
        Parameters(request): Parameters<FetchPaperRequest>,
    ) -> Result<String, ErrorData> {
        let FetchPaperRequest { id, raw, output_path } = request;
        let raw = raw.unwrap_or(false);

        if raw {
            // raw=true: output contains {id, pdf_path}
            let bytes = self.client.fetch_pdf(&id).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to fetch PDF: {}", e), None)
            })?;
            let mut temp_path = std::env::temp_dir();
            temp_path.push(format!("arxiv_{}.pdf", id.replace('/', "_")));
            tokio::fs::write(&temp_path, bytes).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to save PDF: {}", e), None)
            })?;

            // Write metadata to output_path
            let result = serde_json::json!({
                "id": id,
                "pdf_path": temp_path.display().to_string(),
            });
            let json = serde_json::to_string_pretty(&result).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
            })?;
            tokio::fs::write(&output_path, json).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to write to file: {}", e), None)
            })?;

            // Create CypherEngine from the result (wrap in array)
            let json_value = serde_json::json!([result]);
            let engine = CypherEngine::from_json_auto(&json_value).map_err(|e| {
                ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
            })?;

            // Get graph schema
            let graph_schema = engine.get_schema();

            // Store in handler state
            *self.query_engine.write().await = Some(engine);

            let result = serde_json::json!({
                "output_path": output_path,
                "graph_schema": graph_schema
            });

            serde_json::to_string_pretty(&result).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
            })
        } else {
            // raw=false: output contains full paper details
            let paper = self.client.fetch(&id).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to fetch paper: {}", e), None)
            })?;

            // Create CypherEngine from the paper (wrap in array)
            let json_value = serde_json::to_value(&paper).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize paper: {}", e), None)
            })?;
            let paper_array = serde_json::json!([json_value]);

            let engine = CypherEngine::from_json_auto(&paper_array).map_err(|e| {
                ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
            })?;

            // Get graph schema
            let graph_schema = engine.get_schema();

            // Store in handler state
            *self.query_engine.write().await = Some(engine);

            // Write to file
            let json = serde_json::to_string_pretty(&paper).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize paper: {}", e), None)
            })?;
            tokio::fs::write(&output_path, json).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to write to file: {}", e), None)
            })?;

            let result = serde_json::json!({
                "output_path": output_path,
                "graph_schema": graph_schema
            });

            serde_json::to_string_pretty(&result).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
            })
        }
    }

    #[tool(description = "Execute a Cypher query against the loaded search results")]
    pub async fn execute_cypher(
        &self,
        Parameters(request): Parameters<ExecuteCypherRequest>,
    ) -> Result<String, ErrorData> {
        let engine = self.query_engine.read().await;
        match engine.as_ref() {
            Some(e) => {
                let result = e.execute(&request.query).map_err(|e| {
                    ErrorData::internal_error(format!("Query execution failed: {}", e), None)
                })?;
                // Use as_json_array() to get a serializable Value
                let json_value = result.as_json_array();
                serde_json::to_string_pretty(&json_value).map_err(|e| {
                    ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
                })
            }
            None => Err(ErrorData::invalid_params(
                "No search results loaded. Call search_papers first.",
                None,
            )),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ArxivHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: Some(false) }),
                ..Default::default()
            },
            instructions: Some(
                "arXiv MCP Server providing search and fetch capabilities for academic papers."
                    .to_string(),
            ),
            server_info: Implementation {
                name: "arxiv-cli".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
        }
    }

    async fn ping(&self, _ctx: rmcp::service::RequestContext<RoleServer>) -> Result<(), ErrorData> {
        Ok(())
    }

    async fn on_initialized(&self, _ctx: NotificationContext<RoleServer>) {
        // Client initialized successfully
    }
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let client =
        ArxivClient::new(&config).await.map_err(|e| anyhow::anyhow!("Client error: {}", e))?;
    let handler = ArxivHandler::new(client);

    // Serve using stdio transport
    handler
        .serve((stdin(), stdout()))
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server waiting error: {}", e))?;

    Ok(())
}
