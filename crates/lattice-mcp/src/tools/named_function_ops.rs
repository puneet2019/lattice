//! Named function MCP tool handlers.
//!
//! Provides tools to add, remove, and list user-defined named functions
//! (LAMBDA aliases) via the workbook's `NamedFunctionStore`.

use serde::Deserialize;
use serde_json::{Value, json};

use lattice_core::Workbook;

use super::ToolDef;
use crate::schema::{array_prop, object_schema, string_prop};

/// Return tool definitions for named function operations.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "add_named_function".to_string(),
            description: "Define a user-defined named function (LAMBDA alias). Once added, it can be called by name in formulas.".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Function name (must start with a letter or underscore, e.g. 'DOUBLE')")),
                    ("params", array_prop("Parameter names for the lambda (e.g. ['x', 'y'])", json!({"type": "string"}))),
                    ("body", string_prop("Formula body expression (e.g. 'x * 2')")),
                    ("description", string_prop("Optional description of the function")),
                ],
                &["name", "params", "body"],
            ),
        },
        ToolDef {
            name: "remove_named_function".to_string(),
            description: "Remove a user-defined named function by name (case-insensitive).".to_string(),
            input_schema: object_schema(
                &[
                    ("name", string_prop("Name of the function to remove")),
                ],
                &["name"],
            ),
        },
        ToolDef {
            name: "list_named_functions".to_string(),
            description: "List all user-defined named functions in the workbook.".to_string(),
            input_schema: object_schema(&[], &[]),
        },
    ]
}

#[derive(Debug, Deserialize)]
struct AddNamedFunctionArgs {
    name: String,
    params: Vec<String>,
    body: String,
    description: Option<String>,
}

/// Handle the `add_named_function` tool call.
pub fn handle_add_named_function(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: AddNamedFunctionArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    workbook
        .add_named_function(
            &args.name,
            args.params.clone(),
            &args.body,
            args.description.clone(),
        )
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
        "params": args.params,
        "body": args.body,
        "description": args.description,
    }))
}

#[derive(Debug, Deserialize)]
struct RemoveNamedFunctionArgs {
    name: String,
}

/// Handle the `remove_named_function` tool call.
pub fn handle_remove_named_function(workbook: &mut Workbook, args: Value) -> Result<Value, String> {
    let args: RemoveNamedFunctionArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {e}"))?;

    workbook
        .remove_named_function(&args.name)
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "success": true,
        "name": args.name,
    }))
}

/// Handle the `list_named_functions` tool call.
pub fn handle_list_named_functions(workbook: &Workbook) -> Result<Value, String> {
    let functions: Vec<Value> = workbook
        .list_named_functions()
        .iter()
        .map(|nf| {
            json!({
                "name": nf.name,
                "params": nf.params,
                "body": nf.body,
                "description": nf.description,
            })
        })
        .collect();

    Ok(json!({
        "count": functions.len(),
        "named_functions": functions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_named_function() {
        let mut wb = Workbook::new();

        let result = handle_add_named_function(
            &mut wb,
            json!({"name": "DOUBLE", "params": ["x"], "body": "x * 2", "description": "Doubles a value"}),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert_eq!(result["name"], "DOUBLE");
        assert_eq!(result["params"][0], "x");
        assert_eq!(result["body"], "x * 2");
        assert!(wb.get_named_function("DOUBLE").is_some());
    }

    #[test]
    fn test_add_named_function_no_description() {
        let mut wb = Workbook::new();

        let result = handle_add_named_function(
            &mut wb,
            json!({"name": "TRIPLE", "params": ["x"], "body": "x * 3"}),
        )
        .unwrap();

        assert_eq!(result["success"], true);
        assert!(result["description"].is_null());
    }

    #[test]
    fn test_add_named_function_duplicate() {
        let mut wb = Workbook::new();

        handle_add_named_function(
            &mut wb,
            json!({"name": "DOUBLE", "params": ["x"], "body": "x * 2"}),
        )
        .unwrap();

        let result = handle_add_named_function(
            &mut wb,
            json!({"name": "double", "params": ["x"], "body": "x * 3"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_add_named_function_invalid_name() {
        let mut wb = Workbook::new();

        let result = handle_add_named_function(
            &mut wb,
            json!({"name": "1bad", "params": ["x"], "body": "x"}),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_add_named_function_empty_params() {
        let mut wb = Workbook::new();

        let result =
            handle_add_named_function(&mut wb, json!({"name": "BAD", "params": [], "body": "42"}));

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_named_function() {
        let mut wb = Workbook::new();

        handle_add_named_function(
            &mut wb,
            json!({"name": "DOUBLE", "params": ["x"], "body": "x * 2"}),
        )
        .unwrap();

        let result = handle_remove_named_function(&mut wb, json!({"name": "DOUBLE"})).unwrap();

        assert_eq!(result["success"], true);
        assert!(wb.get_named_function("DOUBLE").is_none());
    }

    #[test]
    fn test_remove_named_function_not_found() {
        let mut wb = Workbook::new();

        let result = handle_remove_named_function(&mut wb, json!({"name": "Nothing"}));

        assert!(result.is_err());
    }

    #[test]
    fn test_list_named_functions() {
        let mut wb = Workbook::new();

        handle_add_named_function(
            &mut wb,
            json!({"name": "DOUBLE", "params": ["x"], "body": "x * 2"}),
        )
        .unwrap();
        handle_add_named_function(
            &mut wb,
            json!({"name": "TRIPLE", "params": ["x"], "body": "x * 3"}),
        )
        .unwrap();

        let result = handle_list_named_functions(&wb).unwrap();

        assert_eq!(result["count"], 2);
        let funcs = result["named_functions"].as_array().unwrap();
        assert_eq!(funcs.len(), 2);
    }

    #[test]
    fn test_list_named_functions_empty() {
        let wb = Workbook::new();
        let result = handle_list_named_functions(&wb).unwrap();
        assert_eq!(result["count"], 0);
    }

    #[test]
    fn test_multi_param_function() {
        let mut wb = Workbook::new();

        let result = handle_add_named_function(
            &mut wb,
            json!({
                "name": "ADDMUL",
                "params": ["a", "b", "c"],
                "body": "(a + b) * c",
                "description": "Add a and b, then multiply by c"
            }),
        )
        .unwrap();

        assert_eq!(result["params"].as_array().unwrap().len(), 3);
    }
}
