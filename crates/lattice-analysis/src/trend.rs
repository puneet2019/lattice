//! Trend analysis — linear regression.
//!
//! Fits a line y = slope * x + intercept using ordinary least squares.

use crate::statistics;

/// Perform ordinary least squares linear regression on paired (x, y) data.
///
/// Returns `(slope, intercept)` such that `y ≈ slope * x + intercept`.
///
/// Returns `None` if:
/// - Either slice is empty
/// - The slices have different lengths
/// - All x values are identical (zero variance in x)
///
/// Formula:
/// ```text
/// slope = Σ((xi - x̄)(yi - ȳ)) / Σ((xi - x̄)²)
/// intercept = ȳ - slope * x̄
/// ```
pub fn linear_regression(x: &[f64], y: &[f64]) -> Option<(f64, f64)> {
    if x.is_empty() || y.is_empty() || x.len() != y.len() {
        return None;
    }

    let mean_x = statistics::mean(x)?;
    let mean_y = statistics::mean(y)?;

    let var_x = statistics::variance(x)?;
    if var_x < 1e-15 {
        // All x values are the same — can't fit a line.
        return None;
    }

    let n = x.len() as f64;

    let covariance: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
        .sum::<f64>()
        / n;

    let slope = covariance / var_x;
    let intercept = mean_y - slope * mean_x;

    Some((slope, intercept))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_perfect_line() {
        // y = 2x + 1
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [3.0, 5.0, 7.0, 9.0, 11.0];
        let (slope, intercept) = linear_regression(&x, &y).unwrap();
        assert!(
            (slope - 2.0).abs() < EPSILON,
            "Expected slope=2, got {}",
            slope
        );
        assert!(
            (intercept - 1.0).abs() < EPSILON,
            "Expected intercept=1, got {}",
            intercept
        );
    }

    #[test]
    fn test_horizontal_line() {
        // y = 5 (slope = 0)
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [5.0, 5.0, 5.0, 5.0];
        let (slope, intercept) = linear_regression(&x, &y).unwrap();
        assert!((slope).abs() < EPSILON);
        assert!((intercept - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_negative_slope() {
        // y = -3x + 10
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [10.0, 7.0, 4.0, 1.0];
        let (slope, intercept) = linear_regression(&x, &y).unwrap();
        assert!(
            (slope - (-3.0)).abs() < EPSILON,
            "Expected slope=-3, got {}",
            slope
        );
        assert!(
            (intercept - 10.0).abs() < EPSILON,
            "Expected intercept=10, got {}",
            intercept
        );
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(linear_regression(&[], &[]), None);
    }

    #[test]
    fn test_constant_x() {
        // All x values the same -> undefined.
        assert_eq!(linear_regression(&[3.0, 3.0, 3.0], &[1.0, 2.0, 3.0]), None);
    }

    #[test]
    fn test_two_points() {
        let x = [0.0, 10.0];
        let y = [0.0, 20.0];
        let (slope, intercept) = linear_regression(&x, &y).unwrap();
        assert!((slope - 2.0).abs() < EPSILON);
        assert!((intercept).abs() < EPSILON);
    }
}
