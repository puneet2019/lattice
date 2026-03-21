//! MCP tool registry and dispatch.

pub mod analysis;
pub mod cell_ops;
pub mod chart_ops;
pub mod data_ops;
pub mod file_ops;
pub mod sheet_ops;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Definition of an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// The tool name (e.g. "read_cell").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Registry of available MCP tools.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: Vec<ToolDef>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool definition.
    pub fn register(&mut self, tool: ToolDef) {
        self.tools.push(tool);
    }

    /// Return all registered tool definitions.
    pub fn list(&self) -> &[ToolDef] {
        &self.tools
    }

    /// Find a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Build the default registry with all Lattice tools.
    pub fn default_registry() -> Self {
        let mut reg = Self::new();

        // Cell operations
        for tool in cell_ops::tool_definitions() {
            reg.register(tool);
        }

        // Sheet operations
        for tool in sheet_ops::tool_definitions() {
            reg.register(tool);
        }

        // Data operations (stubs)
        for tool in data_ops::tool_definitions() {
            reg.register(tool);
        }

        // Analysis (stubs)
        for tool in analysis::tool_definitions() {
            reg.register(tool);
        }

        // Chart operations (stubs)
        for tool in chart_ops::tool_definitions() {
            reg.register(tool);
        }

        // File operations (stubs)
        for tool in file_ops::tool_definitions() {
            reg.register(tool);
        }

        reg
    }
}
