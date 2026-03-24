//! Named function management for workbooks.
//!
//! A named function is a user-defined LAMBDA alias stored in the workbook.
//! When the formula evaluator encounters an unrecognized function name, it
//! checks the named function store and, if found, invokes it as a LAMBDA
//! with the provided arguments.
//!
//! Names are looked up case-insensitively and must start with a letter or
//! underscore, followed by alphanumeric characters or underscores.

use std::collections::HashMap;

use crate::error::{LatticeError, Result};

/// A user-defined named function (LAMBDA alias).
#[derive(Debug, Clone, PartialEq)]
pub struct NamedFunction {
    /// The canonical (original-case) name, e.g. `"DOUBLE"`.
    pub name: String,
    /// Parameter names for the lambda, e.g. `["x"]`.
    pub params: Vec<String>,
    /// The formula body, e.g. `"x * 2"`.
    pub body: String,
    /// Optional description for the function.
    pub description: Option<String>,
}

/// A store of named functions with case-insensitive lookup.
#[derive(Debug, Clone, Default)]
pub struct NamedFunctionStore {
    /// Maps lowercase name -> NamedFunction.
    functions: HashMap<String, NamedFunction>,
}

/// Validate that `name` is a legal named-function identifier.
///
/// Rules: must start with a letter or underscore; remaining characters
/// must be alphanumeric or underscore; must not be empty.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(LatticeError::FormulaError(
            "named function name cannot be empty".into(),
        ));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(LatticeError::FormulaError(format!(
            "named function name must start with a letter or underscore, got '{name}'"
        )));
    }

    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Err(LatticeError::FormulaError(format!(
                "named function name contains invalid character '{ch}' in '{name}'"
            )));
        }
    }

    Ok(())
}

impl NamedFunctionStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a named function.
    ///
    /// Returns an error if the name is invalid or already exists
    /// (case-insensitive).
    pub fn add(
        &mut self,
        name: impl Into<String>,
        params: Vec<String>,
        body: impl Into<String>,
        description: Option<String>,
    ) -> Result<()> {
        let name = name.into();
        validate_name(&name)?;

        if params.is_empty() {
            return Err(LatticeError::FormulaError(
                "named function must have at least one parameter".into(),
            ));
        }

        let body = body.into();
        if body.is_empty() {
            return Err(LatticeError::FormulaError(
                "named function body cannot be empty".into(),
            ));
        }

        let key = name.to_lowercase();
        if self.functions.contains_key(&key) {
            return Err(LatticeError::FormulaError(format!(
                "named function '{name}' already exists"
            )));
        }

        self.functions.insert(
            key,
            NamedFunction {
                name,
                params,
                body,
                description,
            },
        );
        Ok(())
    }

    /// Remove a named function by name (case-insensitive).
    ///
    /// Returns an error if the name does not exist.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let key = name.to_lowercase();
        if self.functions.remove(&key).is_none() {
            return Err(LatticeError::FormulaError(format!(
                "named function '{name}' not found"
            )));
        }
        Ok(())
    }

    /// Look up a named function by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&NamedFunction> {
        self.functions.get(&name.to_lowercase())
    }

    /// Return all named functions in arbitrary order.
    pub fn list(&self) -> Vec<&NamedFunction> {
        self.functions.values().collect()
    }

    /// Return the number of named functions in the store.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Return `true` if there are no named functions.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut store = NamedFunctionStore::new();
        store
            .add(
                "DOUBLE",
                vec!["x".into()],
                "x * 2",
                Some("Doubles a value".into()),
            )
            .unwrap();
        let nf = store.get("DOUBLE").unwrap();
        assert_eq!(nf.name, "DOUBLE");
        assert_eq!(nf.params, vec!["x"]);
        assert_eq!(nf.body, "x * 2");
        assert_eq!(nf.description.as_deref(), Some("Doubles a value"));
    }

    #[test]
    fn test_case_insensitive_lookup() {
        let mut store = NamedFunctionStore::new();
        store
            .add("MyFunc", vec!["a".into()], "a + 1", None)
            .unwrap();
        assert!(store.get("myfunc").is_some());
        assert!(store.get("MYFUNC").is_some());
        assert!(store.get("MyFunc").is_some());
    }

    #[test]
    fn test_duplicate_name_rejected() {
        let mut store = NamedFunctionStore::new();
        store
            .add("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        let err = store.add("double", vec!["x".into()], "x * 3", None);
        assert!(err.is_err());
    }

    #[test]
    fn test_remove() {
        let mut store = NamedFunctionStore::new();
        store
            .add("TRIPLE", vec!["x".into()], "x * 3", None)
            .unwrap();
        store.remove("triple").unwrap();
        assert!(store.get("TRIPLE").is_none());
        assert!(store.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_errors() {
        let mut store = NamedFunctionStore::new();
        assert!(store.remove("nothing").is_err());
    }

    #[test]
    fn test_list() {
        let mut store = NamedFunctionStore::new();
        store
            .add("DOUBLE", vec!["x".into()], "x * 2", None)
            .unwrap();
        store
            .add("TRIPLE", vec!["x".into()], "x * 3", None)
            .unwrap();
        assert_eq!(store.list().len(), 2);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_empty_name_rejected() {
        let mut store = NamedFunctionStore::new();
        let result = store.add("", vec!["x".into()], "x + 1", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_name_rejected() {
        let mut store = NamedFunctionStore::new();
        assert!(store.add("1abc", vec!["x".into()], "x", None).is_err());
        assert!(store.add("has space", vec!["x".into()], "x", None).is_err());
        assert!(store.add("no-dash", vec!["x".into()], "x", None).is_err());
    }

    #[test]
    fn test_empty_params_rejected() {
        let mut store = NamedFunctionStore::new();
        let result = store.add("NOOP", vec![], "42", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_body_rejected() {
        let mut store = NamedFunctionStore::new();
        let result = store.add("BAD", vec!["x".into()], "", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_param_function() {
        let mut store = NamedFunctionStore::new();
        store
            .add(
                "ADDMUL",
                vec!["a".into(), "b".into(), "c".into()],
                "(a + b) * c",
                Some("Add a and b, then multiply by c".into()),
            )
            .unwrap();
        let nf = store.get("addmul").unwrap();
        assert_eq!(nf.params.len(), 3);
        assert_eq!(nf.body, "(a + b) * c");
    }
}
