//! Linear interpolation.

use crate::error::Error;

/// Interpolates between two slices element-wise.
pub fn interpolate(from: &[f64], to: &[f64], factor: f64) -> Result<Vec<f64>, Error> {
   if from.len() != to.len() {
      return Err(Error::MismatchedArguments);
   }

   Ok(from
      .iter()
      .zip(to.iter())
      .map(|(&from_val, &to_val)| lerp(from_val, to_val, factor))
      .collect())
}

/// Lerp between two values: `from * (1 - factor) + to * factor`.
pub fn lerp(from: f64, to: f64, factor: f64) -> f64 {
   from.mul_add(1.0 - factor, to * factor)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn interpolate_midpoint() {
      let from = vec![0.0, 10.0, 20.0];
      let to = vec![100.0, 110.0, 120.0];
      let result = interpolate(&from, &to, 0.5).unwrap();
      assert!((result[0] - 50.0).abs() < f64::EPSILON);
      assert!((result[1] - 60.0).abs() < f64::EPSILON);
      assert!((result[2] - 70.0).abs() < f64::EPSILON);
   }

   #[test]
   fn interpolate_at_zero() {
      let from = vec![1.0, 2.0];
      let to = vec![10.0, 20.0];
      let result = interpolate(&from, &to, 0.0).unwrap();
      assert!((result[0] - 1.0).abs() < f64::EPSILON);
      assert!((result[1] - 2.0).abs() < f64::EPSILON);
   }

   #[test]
   fn interpolate_at_one() {
      let from = vec![1.0, 2.0];
      let to = vec![10.0, 20.0];
      let result = interpolate(&from, &to, 1.0).unwrap();
      assert!((result[0] - 10.0).abs() < f64::EPSILON);
      assert!((result[1] - 20.0).abs() < f64::EPSILON);
   }

   #[test]
   fn interpolate_mismatched_lengths() {
      let from = vec![0.0, 10.0];
      let to = vec![100.0, 110.0, 120.0];
      let result = interpolate(&from, &to, 0.5);
      result.unwrap_err();
   }

   #[test]
   fn lerp_values() {
      assert!((lerp(0.0, 100.0, 0.0) - 0.0).abs() < f64::EPSILON);
      assert!((lerp(0.0, 100.0, 0.5) - 50.0).abs() < f64::EPSILON);
      assert!((lerp(0.0, 100.0, 1.0) - 100.0).abs() < f64::EPSILON);
      assert!((lerp(0.0, 100.0, 0.25) - 25.0).abs() < f64::EPSILON);
   }
}
