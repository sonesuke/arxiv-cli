use crate::core::{ArxivClient, Config};
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars::JsonSchema,
    service::NotificationContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use tokio::io::{stdin, stdout};

// Tool request parameter structures

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SearchPapersRequest {
    #[schemars(description = "The search query (e.g., 'quantum computing')")]
    pub query: String,

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

    #[schemars(
        description = "If true, downloads the raw PDF to a local temporary file and returns its path"
    )]
    #[serde(default)]
    pub raw: Option<bool>,
}

pub struct ArxivHandler {
    client: ArxivClient,
    tool_router: ToolRouter<ArxivHandler>,
}

#[tool_router(router = tool_router)]
impl ArxivHandler {
    pub fn new(client: ArxivClient) -> Self {
        Self { client, tool_router: Self::tool_router() }
    }

    #[tool(description = "Search arXiv for papers matching a query")]
    pub async fn search_papers(
        &self,
        Parameters(request): Parameters<SearchPapersRequest>,
    ) -> Result<String, ErrorData> {
        let SearchPapersRequest { query, limit, before, after } = request;

        self.client
            .search(&query, limit, after, before, false)
            .await
            .map(|papers| {
                serde_json::to_string_pretty(&papers)
                    .unwrap_or_else(|_| "Failed to serialize papers".to_string())
            })
            .map_err(|e| ErrorData::internal_error(format!("Failed to search arXiv: {}", e), None))
    }

    #[tool(description = "Fetch details of a specific paper by ID")]
    pub async fn fetch_paper(
        &self,
        Parameters(request): Parameters<FetchPaperRequest>,
    ) -> Result<String, ErrorData> {
        let FetchPaperRequest { id, raw } = request;
        let raw = raw.unwrap_or(false);

        if raw {
            let bytes = self.client.fetch_pdf(&id).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to fetch PDF: {}", e), None)
            })?;
            let mut temp_path = std::env::temp_dir();
            temp_path.push(format!("arxiv_{}.pdf", id.replace('/', "_")));
            tokio::fs::write(&temp_path, bytes).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to save PDF: {}", e), None)
            })?;
            Ok(format!("Successfully downloaded PDF to: {}", temp_path.display()))
        } else {
            self.client
                .fetch(&id)
                .await
                .map(|paper| {
                    serde_json::to_string_pretty(&paper)
                        .unwrap_or_else(|_| "Failed to serialize paper".to_string())
                })
                .map_err(|e| {
                    ErrorData::internal_error(format!("Failed to fetch paper: {}", e), None)
                })
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
