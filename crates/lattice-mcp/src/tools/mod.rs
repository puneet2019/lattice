//! MCP tool registry and dispatch.

pub mod analysis;
pub mod cell_ops;
pub mod chart_ops;
pub mod conditional_format_ops;
pub mod data_ops;
pub mod file_ops;
pub mod filter_view_ops;
pub mod find_replace_ops;
pub mod format_ops;
pub mod formula_ops;
pub mod named_function_ops;
pub mod named_range_ops;
pub mod sheet_ops;
pub mod sparkline_ops;
pub mod validation_ops;

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

        // Data operations
        for tool in data_ops::tool_definitions() {
            reg.register(tool);
        }

        // Analysis
        for tool in analysis::tool_definitions() {
            reg.register(tool);
        }

        // Chart operations
        for tool in chart_ops::tool_definitions() {
            reg.register(tool);
        }

        // Find/replace operations (core-backed)
        for tool in find_replace_ops::tool_definitions() {
            reg.register(tool);
        }

        // Named range operations
        for tool in named_range_ops::tool_definitions() {
            reg.register(tool);
        }

        // Named function operations
        for tool in named_function_ops::tool_definitions() {
            reg.register(tool);
        }

        // Format operations
        for tool in format_ops::tool_definitions() {
            reg.register(tool);
        }

        // Formula operations
        for tool in formula_ops::tool_definitions() {
            reg.register(tool);
        }

        // Validation operations
        for tool in validation_ops::tool_definitions() {
            reg.register(tool);
        }

        // File operations
        for tool in file_ops::tool_definitions() {
            reg.register(tool);
        }

        // Conditional format operations
        for tool in conditional_format_ops::tool_definitions() {
            reg.register(tool);
        }

        // Sparkline operations
        for tool in sparkline_ops::tool_definitions() {
            reg.register(tool);
        }

        // Filter view operations
        for tool in filter_view_ops::tool_definitions() {
            reg.register(tool);
        }

        reg
    }
}

/// Return all tool definitions from every module.
///
/// This is a convenience function for callers that need a flat list of all tools
/// without constructing a `ToolRegistry`.
pub fn tool_definitions() -> Vec<ToolDef> {
    let mut all = Vec::new();
    all.extend(cell_ops::tool_definitions());
    all.extend(sheet_ops::tool_definitions());
    all.extend(data_ops::tool_definitions());
    all.extend(analysis::tool_definitions());
    all.extend(chart_ops::tool_definitions());
    all.extend(find_replace_ops::tool_definitions());
    all.extend(named_range_ops::tool_definitions());
    all.extend(named_function_ops::tool_definitions());
    all.extend(format_ops::tool_definitions());
    all.extend(formula_ops::tool_definitions());
    all.extend(validation_ops::tool_definitions());
    all.extend(file_ops::tool_definitions());
    all.extend(conditional_format_ops::tool_definitions());
    all.extend(sparkline_ops::tool_definitions());
    all.extend(filter_view_ops::tool_definitions());
    all
}
