pub mod integr_mcp;
pub mod tool_mcp;
pub mod session_mcp;

pub use integr_mcp::IntegrationMCP;

pub const MCP_INTEGRATION_SCHEMA: &str = include_str!("mcp_schema.yaml");
