//! Descriptive statistics functions.
//!
//! All functions operate on slices of `f64` values and handle edge cases
//! (empty slices, single elements, NaN) gracefully.

/// Compute the arithmetic mean of a slice.
///
/// Returns `None` if the slice is empty.
pub fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    Some(data.iter().sum::<f64>() / data.len() as f64)
}

/// Compute the median (middle value) of a slice.
///
/// Returns `None` if the slice is empty.
/// For even-length slices, returns the average of the two middle values.
pub fn median(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

/// Compute the population variance of a slice.
///
/// Returns `None` if the slice is empty.
pub fn variance(data: &[f64]) -> Option<f64> {
    let m = mean(data)?;
    let sum_sq: f64 = data.iter().map(|x| (x - m).powi(2)).sum();
    Some(sum_sq / data.len() as f64)
}

/// Compute the population standard deviation of a slice.
///
/// Returns `None` if the slice is empty.
pub fn std_dev(data: &[f64]) -> Option<f64> {
    variance(data).map(|v| v.sqrt())
}

/// Return the minimum value in a slice.
///
/// Returns `None` if the slice is empty.
pub fn min(data: &[f64]) -> Option<f64> {
    data.iter()
        .copied()
        .reduce(|a, b| if a < b { a } else { b })
}

/// Return the maximum value in a slice.
///
/// Returns `None` if the slice is empty.
pub fn max(data: &[f64]) -> Option<f64> {
    data.iter()
        .copied()
        .reduce(|a, b| if a > b { a } else { b })
}

/// Return the number of elements.
pub fn count(data: &[f64]) -> usize {
    data.len()
}

/// Compute the sum of all values.
pub fn sum(data: &[f64]) -> f64 {
    data.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_mean() {
        assert_eq!(mean(&[]), None);
        assert_eq!(mean(&[5.0]), Some(5.0));
        assert!((mean(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 3.0).abs() < EPSILON);
        assert!((mean(&[10.0, 20.0, 30.0]).unwrap() - 20.0).abs() < EPSILON);
    }

    #[test]
    fn test_median_odd() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[7.0]), Some(7.0));
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[5.0, 1.0, 3.0, 4.0, 2.0]), Some(3.0));
    }

    #[test]
    fn test_median_even() {
        assert!((median(&[1.0, 2.0, 3.0, 4.0]).unwrap() - 2.5).abs() < EPSILON);
        assert!((median(&[1.0, 3.0]).unwrap() - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_variance() {
        assert_eq!(variance(&[]), None);
        // All same values -> variance = 0
        assert!((variance(&[5.0, 5.0, 5.0]).unwrap()).abs() < EPSILON);
        // variance of [1, 2, 3, 4, 5] = 2.0 (population)
        assert!((variance(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_std_dev() {
        assert_eq!(std_dev(&[]), None);
        // std_dev of [1, 2, 3, 4, 5] = sqrt(2) ≈ 1.4142
        assert!((std_dev(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap() - 2.0_f64.sqrt()).abs() < EPSILON);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(min(&[]), None);
        assert_eq!(max(&[]), None);
        assert_eq!(min(&[3.0, 1.0, 4.0, 1.5]), Some(1.0));
        assert_eq!(max(&[3.0, 1.0, 4.0, 1.5]), Some(4.0));
        assert_eq!(min(&[-5.0, 0.0, 5.0]), Some(-5.0));
        assert_eq!(max(&[-5.0, 0.0, 5.0]), Some(5.0));
    }

    #[test]
    fn test_count_and_sum() {
        assert_eq!(count(&[]), 0);
        assert_eq!(count(&[1.0, 2.0, 3.0]), 3);
        assert!((sum(&[1.0, 2.0, 3.0]) - 6.0).abs() < EPSILON);
        assert!((sum(&[]) - 0.0).abs() < EPSILON);
    }
}
