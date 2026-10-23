use crate::detector::SuiteDetector;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        CallToolResult, ContentBlock, Implementation, InitializeResult, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct SuiteInfoRequest {}

#[derive(Clone)]
pub struct McpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    detector: SuiteDetector,
}

#[tool_router]
impl McpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            detector: SuiteDetector::new(),
        }
    }

    #[tool(
        description = "Inspect the Bake productivity suite, detecting which CLI utilities (todo, spaghetti, butternut, palmtree) are installed on the user's system, their versions, and paths."
    )]
    async fn bake_suite_info(
        &self,
        _params: Parameters<SuiteInfoRequest>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!("bake_suite_info: scanning system for suite binaries");
        let status = self.detector.detect_suite();
        tracing::info!(
            "bake_suite_info: detected {} / {} tools installed",
            status.installed_count,
            crate::detector::SUITE_TOOLS.len()
        );

        let json_str = serde_json::to_string_pretty(&status)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![ContentBlock::text(json_str)]))
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2025_06_18)
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "I manage the Bake Productivity Suite (todo, palmtree, spaghetti, butternut).
                Start by running `bake_suite_info` to detect which CLI tools are installed and available on this machine."
                    .to_string(),
            )
    }

    async fn initialize(
        &self,
        _request: rmcp::model::InitializeRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> anyhow::Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}
