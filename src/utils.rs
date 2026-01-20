//! Encoding and numeric utilities.

use data_encoding::BASE64;

/// Returns -1.0 for odd numbers, 0.0 for even. Used in bezier control point
/// calculation.
pub const fn odd_coefficient(num: usize) -> f64 {
   if num % 2 == 1 { -1.0 } else { 0.0 }
}

/// Rounds using JavaScript's `Math.round()` semantics.
///
/// JavaScript rounds -0.5 to 0 (toward positive infinity), while Rust rounds
/// -0.5 to -1 (away from zero). This matches JavaScript.
#[expect(clippy::float_cmp, reason = "checking for exact -0.5 boundary case")]
pub fn js_round(num: f64) -> f64 {
   let decimal_part = num - num.trunc();
   if decimal_part == -0.5 {
      num.ceil()
   } else {
      num.round()
   }
}

/// Converts a non-negative float to hex (e.g., 10.0 -> "A", 0.5 -> "0.8").
#[expect(
   clippy::cast_possible_truncation,
   reason = "working with small display values"
)]
#[expect(clippy::cast_sign_loss, reason = "values are validated to be positive")]
#[expect(
   clippy::cast_precision_loss,
   reason = "acceptable for display purposes"
)]
#[expect(
   clippy::modulo_arithmetic,
   reason = "quotient is always positive in the loop"
)]
#[expect(
   clippy::while_float,
   reason = "intentionally iterating on diminishing fraction"
)]
pub fn float_to_hex(value: f64) -> String {
   if value == 0.0 {
      return "0".to_owned();
   }

   let mut result = String::new();
   let mut quotient = value.floor() as i64;
   let mut fraction = value - quotient as f64;

   let digit_to_char = |digit: i64| -> char {
      if digit > 9 {
         char::from_u32((digit as u32) + 55).unwrap_or('?')
      } else {
         char::from_digit(digit as u32, 10).unwrap_or('?')
      }
   };

   if quotient == 0 {
      result.push('0');
   } else {
      while quotient > 0 {
         let remainder = quotient % 16;
         quotient /= 16;
         result.insert(0, digit_to_char(remainder));
      }
   }

   if fraction > 0.0 {
      result.push('.');
      while fraction > 0.0 {
         fraction *= 16.0;
         let integer_part = fraction.floor() as i64;
         fraction -= integer_part as f64;
         result.push(digit_to_char(integer_part));
         if result.len() > 20 {
            break;
         }
      }
   }

   result
}

pub fn base64_encode(data: &[u8]) -> String {
   BASE64.encode(data)
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, data_encoding::DecodeError> {
   BASE64.decode(input.as_bytes())
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn odd_coefficient_values() {
      assert!((odd_coefficient(0) - 0.0).abs() < f64::EPSILON);
      assert!((odd_coefficient(1) - (-1.0)).abs() < f64::EPSILON);
      assert!((odd_coefficient(2) - 0.0).abs() < f64::EPSILON);
      assert!((odd_coefficient(3) - (-1.0)).abs() < f64::EPSILON);
      assert!((odd_coefficient(100) - 0.0).abs() < f64::EPSILON);
      assert!((odd_coefficient(101) - (-1.0)).abs() < f64::EPSILON);
   }

   #[test]
   fn js_round_positive_values() {
      assert!((js_round(0.0) - 0.0).abs() < f64::EPSILON);
      assert!((js_round(0.4) - 0.0).abs() < f64::EPSILON);
      assert!((js_round(0.5) - 1.0).abs() < f64::EPSILON);
      assert!((js_round(0.6) - 1.0).abs() < f64::EPSILON);
      assert!((js_round(1.5) - 2.0).abs() < f64::EPSILON);
   }

   #[test]
   fn js_round_negative_values() {
      assert!((js_round(-0.4) - 0.0).abs() < f64::EPSILON);
      assert!((js_round(-0.5) - 0.0).abs() < f64::EPSILON);
      assert!((js_round(-0.6) - (-1.0)).abs() < f64::EPSILON);
      assert!((js_round(-1.5) - (-1.0)).abs() < f64::EPSILON);
   }

   #[test]
   fn float_to_hex_integers() {
      assert_eq!(float_to_hex(0.0), "0");
      assert_eq!(float_to_hex(10.0), "A");
      assert_eq!(float_to_hex(15.0), "F");
      assert_eq!(float_to_hex(16.0), "10");
      assert_eq!(float_to_hex(255.0), "FF");
   }

   #[test]
   fn float_to_hex_fractions() {
      assert_eq!(float_to_hex(0.5), "0.8");
      assert_eq!(float_to_hex(0.25), "0.4");
   }

   #[test]
   fn base64_roundtrip() {
      let original = b"The quick brown fox jumps over the lazy dog";
      let encoded = base64_encode(original);
      let decoded = base64_decode(&encoded).unwrap();
      assert_eq!(decoded, original);
   }

   #[test]
   fn base64_empty() {
      let encoded = base64_encode(b"");
      assert_eq!(encoded, "");
      let decoded = base64_decode("").unwrap();
      assert!(decoded.is_empty());
   }

   #[test]
   fn base64_invalid() {
      base64_decode("not valid base64!!!").unwrap_err();
   }
}
