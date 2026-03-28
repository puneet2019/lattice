//! Read charts from `.xlsx` files.
//!
//! This module provides `read_xlsx_charts` which opens an xlsx ZIP archive,
//! discovers chart XML entries via OPC relationships, and parses each chart
//! using the parser in `xlsx_chart_parser`.
//!
//! The zip extraction and relationship resolution layer will be added
//! in a follow-up commit.  For now this module re-exports the core types.

pub use crate::xlsx_chart_parser::ImportedChart;
