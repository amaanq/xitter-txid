//! 2D rotation matrix.

/// Converts degrees to a 2x2 rotation matrix: `[cos, -sin, sin, cos]`.
pub fn rotation_matrix(degrees: f64) -> [f64; 4] {
   let radians = degrees.to_radians();
   let cos = radians.cos();
   let sin = radians.sin();
   [cos, -sin, sin, cos]
}

#[cfg(test)]
mod tests {
   use super::*;

   const TOLERANCE: f64 = 0.00001;

   #[test]
   fn rotation_90_degrees() {
      let matrix = rotation_matrix(90.0);
      assert!((matrix[0] - 0.0).abs() < TOLERANCE);
      assert!((matrix[1] - (-1.0)).abs() < TOLERANCE);
      assert!((matrix[2] - 1.0).abs() < TOLERANCE);
      assert!((matrix[3] - 0.0).abs() < TOLERANCE);
   }

   #[test]
   fn rotation_0_degrees() {
      let matrix = rotation_matrix(0.0);
      assert!((matrix[0] - 1.0).abs() < TOLERANCE);
      assert!((matrix[1] - 0.0).abs() < TOLERANCE);
      assert!((matrix[2] - 0.0).abs() < TOLERANCE);
      assert!((matrix[3] - 1.0).abs() < TOLERANCE);
   }

   #[test]
   fn rotation_180_degrees() {
      let matrix = rotation_matrix(180.0);
      assert!((matrix[0] - (-1.0)).abs() < TOLERANCE);
      assert!((matrix[1] - 0.0).abs() < TOLERANCE);
      assert!((matrix[2] - 0.0).abs() < TOLERANCE);
      assert!((matrix[3] - (-1.0)).abs() < TOLERANCE);
   }
}
