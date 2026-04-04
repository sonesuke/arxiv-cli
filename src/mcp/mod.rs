use crate::core::{ArxivClient, Config};
use cypher_rs::CypherEngine;
use lru::LruCache;
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo, ToolsCapability},
    schemars::JsonSchema,
    service::NotificationContext,
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tokio::sync::RwLock;

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

    #[schemars(
        description = "Filter by arXiv category (e.g., 'cs.AI', 'physics.quant-ph', 'math.NA')"
    )]
    #[serde(default)]
    pub category: Option<String>,
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

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ExecuteCypherRequest {
    #[schemars(description = "Dataset name to query")]
    pub dataset: String,

    #[schemars(description = "Cypher query to execute")]
    pub query: String,
}

/// Cache key for search queries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SearchCacheKey {
    query: String,
    limit: Option<usize>,
    before: Option<String>,
    after: Option<String>,
    category: Option<String>,
}

impl SearchCacheKey {
    fn from_request(req: &SearchPapersRequest) -> Self {
        Self {
            query: req.query.clone(),
            limit: req.limit,
            before: req.before.clone(),
            after: req.after.clone(),
            category: req.category.clone(),
        }
    }

    /// Generate a unique dataset name from this cache key
    fn to_dataset_name(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        format!("search_{:x}", hasher.finish())
    }
}

/// Cache key for fetch queries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FetchCacheKey {
    id: String,
    raw: bool,
}

impl FetchCacheKey {
    fn from_request(req: &FetchPaperRequest) -> Self {
        Self { id: req.id.clone(), raw: req.raw.unwrap_or(false) }
    }

    /// Generate a unique dataset name from this cache key
    fn to_dataset_name(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        format!("fetch_{:x}", hasher.finish())
    }
}

pub struct ArxivHandler {
    client: ArxivClient,
    tool_router: ToolRouter<ArxivHandler>,
    query_engines: Arc<RwLock<HashMap<String, CypherEngine>>>,
    // Cache: search query key -> dataset name (LRU 100 entries)
    search_cache: Arc<RwLock<LruCache<SearchCacheKey, String>>>,
    // Cache: fetch query key -> dataset name (LRU 100 entries)
    fetch_cache: Arc<RwLock<LruCache<FetchCacheKey, String>>>,
}

#[tool_router(router = tool_router)]
impl ArxivHandler {
    pub fn new(client: ArxivClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
            query_engines: Arc::new(RwLock::new(HashMap::new())),
            search_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(),
            ))),
            fetch_cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(),
            ))),
        }
    }

    #[tool(description = "Search arXiv for papers matching a query")]
    pub async fn search_papers(
        &self,
        Parameters(request): Parameters<SearchPapersRequest>,
    ) -> Result<String, ErrorData> {
        // Generate cache key and dataset name
        let cache_key = SearchCacheKey::from_request(&request);
        let dataset = cache_key.to_dataset_name();

        // Check cache for existing dataset with same query parameters
        let cached_dataset = {
            let mut cache = self.search_cache.write().await;
            cache.get(&cache_key).cloned()
        };

        if let Some(cached) = cached_dataset {
            // Return cached dataset
            let engines = self.query_engines.read().await;
            if let Some(engine) = engines.get(&cached) {
                let graph_schema = engine.get_schema();
                let result = serde_json::json!({
                    "dataset": cached,
                    "count": "cached",
                    "graph_schema": graph_schema
                });
                return serde_json::to_string_pretty(&result).map_err(|e| {
                    ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
                });
            }
        }

        // Not cached, perform the search
        let papers = self
            .client
            .search(
                &request.query,
                request.limit,
                request.after.clone(),
                request.before.clone(),
                request.category.clone(),
                false,
            )
            .await
            .map_err(|e| {
                ErrorData::internal_error(format!("Failed to search arXiv: {}", e), None)
            })?;

        // Return early if no results — CypherEngine cannot build from empty array
        if papers.is_empty() {
            let result = serde_json::json!({
                "dataset": dataset,
                "count": 0,
                "message": "No papers found matching the query"
            });
            return serde_json::to_string_pretty(&result).map_err(|e| {
                ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
            });
        }

        // Create CypherEngine with auto-detection
        // Wrap in object so cypher-rs can detect a named node path
        let json_value = serde_json::json!({
            "papers": &papers
        });

        let engine = CypherEngine::from_json_auto(&json_value).map_err(|e| {
            ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
        })?;

        // Get graph schema from CypherEngine
        let graph_schema = engine.get_schema();

        // Store in handler state with dataset name as key
        self.query_engines.write().await.insert(dataset.clone(), engine);

        // Update cache
        let mut cache = self.search_cache.write().await;
        cache.put(cache_key, dataset.clone());

        let result = serde_json::json!({
            "dataset": dataset,
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
        let raw = request.raw.unwrap_or(false);

        // Generate cache key and dataset name
        let cache_key = FetchCacheKey::from_request(&request);
        let dataset = cache_key.to_dataset_name();

        // Check cache for existing dataset with same fetch parameters
        let cached_dataset = {
            let mut cache = self.fetch_cache.write().await;
            cache.get(&cache_key).cloned()
        };

        if let Some(cached) = cached_dataset {
            // Return cached dataset
            let engines = self.query_engines.read().await;
            if let Some(engine) = engines.get(&cached) {
                let graph_schema = engine.get_schema();
                let result = serde_json::json!({
                    "dataset": cached,
                    "graph_schema": graph_schema
                });
                return serde_json::to_string_pretty(&result).map_err(|e| {
                    ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
                });
            }
        }

        // Not cached, perform the fetch
        let engine = if raw {
            // raw=true: output contains {id, pdf_path}
            let bytes = self.client.fetch_pdf(&request.id).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to fetch PDF: {}", e), None)
            })?;
            let mut temp_path = std::env::temp_dir();
            temp_path.push(format!("arxiv_{}.pdf", request.id.replace('/', "_")));
            tokio::fs::write(&temp_path, bytes).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to save PDF: {}", e), None)
            })?;

            // Create CypherEngine from the result (wrap in object with named key)
            let json_value = serde_json::json!({
                "results": [{
                    "id": request.id,
                    "pdf_path": temp_path.display().to_string(),
                }]
            });
            CypherEngine::from_json_auto(&json_value).map_err(|e| {
                ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
            })?
        } else {
            // raw=false: output contains full paper details
            let paper = self.client.fetch(&request.id).await.map_err(|e| {
                ErrorData::internal_error(format!("Failed to fetch paper: {}", e), None)
            })?;

            // Create CypherEngine from the paper (wrap in object with named key)
            let json_value = serde_json::json!({
                "papers": [&paper]
            });

            CypherEngine::from_json_auto(&json_value).map_err(|e| {
                ErrorData::internal_error(format!("Failed to create query engine: {}", e), None)
            })?
        };

        // Get graph schema
        let graph_schema = engine.get_schema();

        // Store in handler state with dataset name as key
        self.query_engines.write().await.insert(dataset.clone(), engine);

        // Update cache
        let mut cache = self.fetch_cache.write().await;
        cache.put(cache_key, dataset.clone());

        let result = serde_json::json!({
            "dataset": dataset,
            "graph_schema": graph_schema
        });

        serde_json::to_string_pretty(&result).map_err(|e| {
            ErrorData::internal_error(format!("Failed to serialize result: {}", e), None)
        })
    }

    #[tool(description = "Execute a Cypher query against the loaded search results")]
    pub async fn execute_cypher(
        &self,
        Parameters(request): Parameters<ExecuteCypherRequest>,
    ) -> Result<String, ErrorData> {
        let engines = self.query_engines.read().await;
        match engines.get(&request.dataset) {
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
                format!(
                    "Dataset '{}' not found. Call search_papers or fetch_paper first.",
                    request.dataset
                ),
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
