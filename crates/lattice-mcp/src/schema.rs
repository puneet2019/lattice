//! JSON Schema helpers for MCP tool input definitions.

use serde_json::{Value, json};

/// Build a JSON Schema object with type "object" and the given properties.
///
/// # Example
/// ```
/// use lattice_mcp::schema::object_schema;
/// use serde_json::json;
///
/// let schema = object_schema(
///     &[
///         ("sheet", json!({"type": "string", "description": "Sheet name"})),
///         ("cell_ref", json!({"type": "string", "description": "Cell in A1 notation"})),
///     ],
///     &["sheet", "cell_ref"],
/// );
/// ```
pub fn object_schema(properties: &[(&str, Value)], required: &[&str]) -> Value {
    let mut props = serde_json::Map::new();
    for (name, schema) in properties {
        props.insert(name.to_string(), schema.clone());
    }

    json!({
        "type": "object",
        "properties": props,
        "required": required,
    })
}

/// Create a simple string property schema.
pub fn string_prop(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description,
    })
}

/// Create a number property schema.
pub fn number_prop(description: &str) -> Value {
    json!({
        "type": "number",
        "description": description,
    })
}

/// Create a boolean property schema.
pub fn bool_prop(description: &str) -> Value {
    json!({
        "type": "boolean",
        "description": description,
    })
}

/// Create an array property schema with items of a given type.
pub fn array_prop(description: &str, items: Value) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_schema() {
        let schema = object_schema(
            &[
                ("name", string_prop("The name")),
                ("age", number_prop("The age")),
            ],
            &["name"],
        );
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["name"]["type"], "string");
        assert_eq!(schema["required"][0], "name");
    }
}
