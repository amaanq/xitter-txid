//! Cubic bezier curve for animation timing.

/// Cubic bezier curve, like CSS `cubic-bezier()`.
pub struct Cubic {
   curves: Vec<f64>,
}

impl Cubic {
   pub const fn new(curves: Vec<f64>) -> Self {
      Self { curves }
   }

   /// Returns the Y value for a given X (time) using binary search.
   #[expect(clippy::float_cmp, reason = "checking boundary conditions")]
   pub fn value(&self, time: f64) -> f64 {
      // Linear extrapolation outside [0, 1]
      if time <= 0.0 {
         let start_gradient = if self.curves[0] > 0.0 {
            self.curves[1] / self.curves[0]
         } else if self.curves[1] == 0.0 && self.curves[2] > 0.0 {
            self.curves[3] / self.curves[2]
         } else {
            0.0
         };
         return start_gradient * time;
      }

      if time >= 1.0 {
         let end_gradient = if self.curves[2] < 1.0 {
            (self.curves[3] - 1.0) / (self.curves[2] - 1.0)
         } else if self.curves[2] == 1.0 && self.curves[0] < 1.0 {
            (self.curves[1] - 1.0) / (self.curves[0] - 1.0)
         } else {
            0.0
         };
         return 1.0 + end_gradient * (time - 1.0);
      }

      // Binary search for the parameter that gives us target X
      let mut low = 0.0_f64;
      let mut high = 1.0_f64;
      let mut mid;

      loop {
         mid = f64::midpoint(low, high);
         let x_estimate = Self::bezier(self.curves[0], self.curves[2], mid);

         if (time - x_estimate).abs() < 0.00001 {
            return Self::bezier(self.curves[1], self.curves[3], mid);
         }

         if (high - low).abs() < f64::EPSILON {
            break;
         }

         if x_estimate < time {
            low = mid;
         } else {
            high = mid;
         }
      }

      Self::bezier(self.curves[1], self.curves[3], mid)
   }

   /// Bezier formula: 3*p1*(1-t)²*t + 3*p2*(1-t)*t² + t³.
   fn bezier(p1: f64, p2: f64, param: f64) -> f64 {
      let complement = 1.0 - param;
      let complement_sq = complement * complement;
      let param_sq = param * param;

      (3.0 * p1 * complement_sq).mul_add(
         param,
         (3.0 * p2 * complement).mul_add(param_sq, param_sq * param),
      )
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn cubic_curve_value() {
      let cubic = Cubic::new(vec![0.1, 0.2, 0.3, 0.4]);
      let value = cubic.value(0.5);
      assert!(value > 0.0);
   }

   #[test]
   fn cubic_curve_boundaries() {
      let cubic = Cubic::new(vec![0.25, 0.1, 0.25, 1.0]);
      assert!((cubic.value(0.0) - 0.0).abs() < f64::EPSILON);
      assert!((cubic.value(1.0) - 1.0).abs() < 0.001);
   }

   #[test]
   fn cubic_curve_extrapolation() {
      // Curve with non-zero gradients at boundaries
      let cubic = Cubic::new(vec![0.4, 0.2, 0.6, 0.8]);
      let below = cubic.value(-0.1);
      let above = cubic.value(1.1);
      assert!(below < 0.0);
      assert!(above > 1.0);
   }
}
