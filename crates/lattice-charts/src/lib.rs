//! Chart engine for Lattice spreadsheets.
//!
//! Provides chart definitions, data models, and SVG rendering for
//! bar, line, pie, scatter, and area charts.

pub mod chart;
pub mod render;
pub mod svg;
pub mod types;

pub use chart::{Chart, ChartData, ChartOptions, ChartType, DataSeries};
pub use render::{render_chart, render_to_svg};
