//! Correlation analysis.
//!
//! Computes the Pearson correlation coefficient between two data series.

use crate::statistics;

/// Compute the Pearson correlation coefficient between two equal-length slices.
///
/// Returns a value in the range `[-1.0, 1.0]`:
/// - `1.0` = perfect positive correlation
/// - `0.0` = no correlation
/// - `-1.0` = perfect negative correlation
///
/// Returns `None` if:
/// - Either slice is empty
/// - The slices have different lengths
/// - Either series has zero variance (all identical values)
///
/// Formula: r = Σ((xi - x̄)(yi - ȳ)) / (n * σx * σy)
pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return None;
    }

    let n = x.len() as f64;
    let mean_x = statistics::mean(x)?;
    let mean_y = statistics::mean(y)?;

    let std_x = statistics::std_dev(x)?;
    let std_y = statistics::std_dev(y)?;

    // Guard against zero standard deviation (constant series).
    if std_x < 1e-15 || std_y < 1e-15 {
        return None;
    }

    let covariance: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum::<f64>()
        / n;

    Some(covariance / (std_x * std_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_perfect_positive_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 6.0, 8.0, 10.0];
        let r = pearson_correlation(&x, &y).unwrap();
        assert!((r - 1.0).abs() < EPSILON, "Expected 1.0, got {}", r);
    }

    #[test]
    fn test_perfect_negative_correlation() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [10.0, 8.0, 6.0, 4.0, 2.0];
        let r = pearson_correlation(&x, &y).unwrap();
        assert!((r - (-1.0)).abs() < EPSILON, "Expected -1.0, got {}", r);
    }

    #[test]
    fn test_no_correlation() {
        // These values have approximately zero correlation.
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [2.0, 4.0, 1.0, 5.0, 3.0];
        let r = pearson_correlation(&x, &y).unwrap();
        // Not exactly 0, but should be close to 0.
        assert!(r.abs() < 0.5, "Expected near 0, got {}", r);
    }

    #[test]
    fn test_empty_slices() {
        assert_eq!(pearson_correlation(&[], &[]), None);
        assert_eq!(pearson_correlation(&[1.0], &[]), None);
    }

    #[test]
    fn test_different_lengths() {
        assert_eq!(pearson_correlation(&[1.0, 2.0], &[1.0, 2.0, 3.0]), None);
    }

    #[test]
    fn test_constant_series() {
        // Zero variance -> undefined correlation.
        assert_eq!(
            pearson_correlation(&[5.0, 5.0, 5.0], &[1.0, 2.0, 3.0]),
            None
        );
    }
}
