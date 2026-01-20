//! Transaction ID generation for X (Twitter) API requests.

use std::{
   iter,
   time::{
      SystemTime,
      UNIX_EPOCH,
   },
};

use hmac_sha256::Hash;

use crate::{
   cubic_curve::Cubic,
   error::Error,
   interpolate::interpolate,
   rotation::rotation_matrix,
   utils::{
      base64_decode,
      base64_encode,
      float_to_hex,
      js_round,
      odd_coefficient,
   },
};

#[cfg(feature = "fetch")]
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, \
                          like Gecko) Chrome/133.0.0.0 Safari/537.36";

const ONDEMAND_BASE_URL: &str = "https://abs.twimg.com/responsive-web/client-web";

/// Secret salt from X's client-side JavaScript.
const HASH_SALT: &str = "obfiowerehiring";

/// X's custom epoch: 2023-05-01 00:00:00 UTC.
const X_EPOCH: u64 = 1_682_924_400;

const TOTAL_ANIMATION_TIME: f64 = 4096.0;
const FRAME_COUNT: u8 = 4;
const ROW_INDEX_MODULUS: u8 = 16;
const FRAME_SELECTOR_INDEX: usize = 5;
const MIN_FRAME_VALUES: usize = 11;

/// Client for generating X (Twitter) transaction IDs.
///
/// Holds cryptographic material extracted from X's homepage and JavaScript
/// files. Create one with [`fetch`](Self::fetch) (requires `fetch` feature)
/// or [`new`](Self::new).
pub struct ClientTransaction {
   key_bytes:     Vec<u8>,
   animation_key: String,
}

impl ClientTransaction {
   /// Fetches X.com and creates a ready-to-use client.
   ///
   /// ```ignore
   /// let client = ClientTransaction::fetch()?;
   /// let id = client.generate_transaction_id("GET", "/i/api/1.1/jot/client_event.json");
   /// ```
   #[cfg(feature = "fetch")]
   pub fn fetch() -> Result<Self, Error> {
      let home_response = minreq::get("https://x.com")
         .with_header("User-Agent", USER_AGENT)
         .send()?;

      if home_response.status_code != 200 {
         return Err(Error::HttpStatus(home_response.status_code, "x.com"));
      }

      let home_html = home_response.as_str()?.to_owned();
      let ondemand_url = Self::extract_ondemand_url(&home_html)?;

      let js_response = minreq::get(&ondemand_url)
         .with_header("User-Agent", USER_AGENT)
         .send()?;

      if js_response.status_code != 200 {
         return Err(Error::HttpStatus(js_response.status_code, "ondemand.js"));
      }

      let ondemand_js = js_response.as_str()?.to_owned();
      Self::new(&home_html, &ondemand_js)
   }

   /// Creates a client from pre-fetched HTML and JavaScript.
   ///
   /// Use this if you want to bring your own HTTP client.
   /// Get the JS URL with [`extract_ondemand_url`](Self::extract_ondemand_url).
   pub fn new(home_page_html: &str, ondemand_js: &str) -> Result<Self, Error> {
      let (row_index, key_bytes_indices) = Self::parse_indices(ondemand_js)?;
      let key = Self::verification_key(home_page_html)?;
      let key_bytes = Self::decode_key(&key)?;
      let animation_key =
         Self::compute_animation_key(&key_bytes, home_page_html, row_index, &key_bytes_indices)?;

      Ok(Self {
         key_bytes,
         animation_key,
      })
   }

   /// Extracts the ondemand.s.*.js URL from homepage HTML.
   pub fn extract_ondemand_url(home_page_html: &str) -> Result<String, Error> {
      let markers = ["\"ondemand.s\"", "'ondemand.s'"];

      for marker in markers {
         if let Some(pos) = home_page_html.find(marker) {
            let Some(after_marker) = home_page_html.get(pos + marker.len()..) else {
               continue;
            };

            let after_colon = after_marker
               .trim_start()
               .strip_prefix(':')
               .map(str::trim_start);

            if let Some(rest) = after_colon {
               let (quote, rest) = if let Some(stripped) = rest.strip_prefix('"') {
                  ('"', stripped)
               } else if let Some(stripped) = rest.strip_prefix('\'') {
                  ('\'', stripped)
               } else {
                  continue;
               };

               if let Some(end) = rest.find(quote) {
                  let hash = &rest[..end];
                  if !hash.is_empty() && hash.chars().all(char::is_alphanumeric) {
                     return Ok(format!("{ONDEMAND_BASE_URL}/ondemand.s.{hash}a.js"));
                  }
               }
            }
         }
      }

      // Provide helpful context about what we received
      let hint = if home_page_html.contains("login") || home_page_html.contains("LoginForm") {
         " (received login page - may need cookies)"
      } else if home_page_html.len() < 10000 {
         " (response too small - may be rate limited or blocked)"
      } else {
         " (X may have changed their page structure)"
      };
      Err(Error::MissingKey(format!("ondemand file hash{hint}")))
   }

   /// Generates a transaction ID for an API request.
   #[must_use]
   pub fn generate_transaction_id(&self, method: &str, path: &str) -> String {
      let time = Self::current_time();
      let hash_input = format!("{method}!{path}!{time}{HASH_SALT}{}", self.animation_key);
      let hash_result = Hash::hash(hash_input.as_bytes());

      let time_bytes = time.to_le_bytes();
      let random_byte = hash_result[16];

      let mut bytes = Vec::with_capacity(self.key_bytes.len() + 4 + 16 + 1);
      bytes.extend_from_slice(&self.key_bytes);
      bytes.extend_from_slice(&time_bytes);
      bytes.extend_from_slice(&hash_result[..16]);
      bytes.push(3); // protocol version

      let mut encoded = vec![random_byte];
      encoded.extend(bytes.iter().map(|&byte| byte ^ random_byte));

      base64_encode(&encoded).trim_end_matches('=').to_owned()
   }

   #[expect(
      clippy::cast_possible_truncation,
      reason = "timestamp fits in u32 until 2159"
   )]
   fn current_time() -> u32 {
      SystemTime::now()
         .duration_since(UNIX_EPOCH)
         .map(|duration| duration.as_secs().saturating_sub(X_EPOCH) as u32)
         .unwrap_or(0)
   }

   /// Finds (e[N], 16) patterns in the JS to get key byte indices.
   fn parse_indices(ondemand_js: &str) -> Result<(usize, Vec<usize>), Error> {
      let mut indices = Vec::new();
      let bytes = ondemand_js.as_bytes();

      for idx in 0..bytes.len().saturating_sub(10) {
         if bytes.get(idx) == Some(&b'(') && bytes.get(idx + 2) == Some(&b'[') {
            let start = idx + 3;
            let mut end = start;
            while end < bytes.len() && bytes.get(end).is_some_and(u8::is_ascii_digit) {
               end += 1;
            }

            if end > start
               && let Some(rest) = ondemand_js.get(end..)
               && (rest.starts_with("], 16)") || rest.starts_with("],16)"))
               && let Some(num_str) = ondemand_js.get(start..end)
               && let Ok(num) = num_str.parse::<usize>()
            {
               indices.push(num);
            }
         }
      }

      if indices.is_empty() {
         return Err(Error::MissingKey("key byte indices".into()));
      }

      Ok((indices[0], indices[1..].to_vec()))
   }

   fn verification_key(html: &str) -> Result<String, Error> {
      let marker = "name=\"twitter-site-verification\"";
      let pos = html
         .find(marker)
         .ok_or_else(|| Error::MissingKey("twitter-site-verification meta tag".into()))?;

      let tag_start = html
         .get(..pos)
         .and_then(|slice| slice.rfind('<'))
         .unwrap_or(0);

      let tag_end = html
         .get(pos..)
         .and_then(|slice| slice.find('>'))
         .map_or(html.len(), |offset| pos + offset);

      let tag = html
         .get(tag_start..tag_end)
         .ok_or_else(|| Error::Parse("malformed meta tag".into()))?;

      let content_marker = "content=\"";
      let content_pos = tag
         .find(content_marker)
         .ok_or_else(|| Error::MissingKey("content attribute".into()))?;

      let value_start = content_pos + content_marker.len();
      let value_end = tag
         .get(value_start..)
         .and_then(|slice| slice.find('"'))
         .ok_or_else(|| Error::Parse("malformed content attribute".into()))?;

      tag.get(value_start..value_start + value_end)
         .map(ToOwned::to_owned)
         .ok_or_else(|| Error::Parse("could not extract verification key".into()))
   }

   fn decode_key(key: &str) -> Result<Vec<u8>, Error> {
      base64_decode(key).map_err(Error::from)
   }

   fn animation_frames(html: &str) -> Vec<String> {
      let mut frames = Vec::new();
      let mut search_pos = 0;

      while let Some(pos) = html
         .get(search_pos..)
         .and_then(|slice| slice.find("id=\"loading-x-anim"))
      {
         let abs_pos = search_pos + pos;

         let Some(svg_content) = html
            .get(abs_pos..)
            .and_then(|slice| slice.find("</svg>").map(|end| &slice[..end]))
         else {
            break;
         };

         let mut path_search = 0;
         let mut found_curve_path = false;

         while let Some(path_pos) = svg_content
            .get(path_search..)
            .and_then(|slice| slice.find("<path"))
         {
            let path_abs = path_search + path_pos;

            let path_end = svg_content
               .get(path_abs..)
               .and_then(|slice| slice.find("/>").or_else(|| slice.find("></path>")));

            if let Some(end_offset) = path_end
               && let Some(path_tag) = svg_content.get(path_abs..path_abs + end_offset)
               && let Some(d_value) = Self::extract_path_d(path_tag)
               && d_value.contains('C')
               && !found_curve_path
            {
               frames.push(d_value.to_owned());
               found_curve_path = true;
            }
            path_search = path_abs + 5;
         }

         search_pos = abs_pos + svg_content.len();
      }

      frames
   }

   fn extract_path_d(path_tag: &str) -> Option<&str> {
      let d_pos = path_tag.find(" d=\"")?;
      let d_start = d_pos + 4;
      let rest = path_tag.get(d_start..)?;
      let d_end = rest.find('"')?;
      rest.get(..d_end)
   }

   fn parse_path_to_coordinates(path_d: &str) -> Vec<Vec<i32>> {
      // Skip initial move command ("M0 0 0 0" is 9 chars)
      let d_content = path_d.get(9..).unwrap_or(path_d);

      d_content
         .split('C')
         .map(|segment| {
            segment
               .replace(|chr: char| !chr.is_ascii_digit() && chr != '-', " ")
               .split_whitespace()
               .filter_map(|token| token.parse::<i32>().ok())
               .collect()
         })
         .collect()
   }

   fn frame_data(key_bytes: &[u8], html: &str) -> Result<Vec<Vec<i32>>, Error> {
      let frames = Self::animation_frames(html);

      if frames.is_empty() {
         return Err(Error::MissingKey("animation frames".into()));
      }

      let selector_byte = key_bytes
         .get(FRAME_SELECTOR_INDEX)
         .ok_or_else(|| Error::Parse("key too short for frame selection".into()))?;

      let frame_index = usize::from(selector_byte % FRAME_COUNT);
      let frame = frames
         .get(frame_index)
         .ok_or_else(|| Error::Parse("frame index out of bounds".into()))?;

      Ok(Self::parse_path_to_coordinates(frame))
   }

   fn solve(value: f64, min_val: f64, max_val: f64, rounding: bool) -> f64 {
      let result = value.mul_add((max_val - min_val) / 255.0, min_val);
      if rounding {
         result.floor()
      } else {
         (result * 100.0).round() / 100.0
      }
   }

   #[expect(
      clippy::cast_possible_truncation,
      reason = "color values are clamped to 0-255"
   )]
   #[expect(
      clippy::missing_asserts_for_indexing,
      reason = "length check at function start ensures indices are valid"
   )]
   fn animate(frames: &[i32], target_time: f64) -> Result<String, Error> {
      if frames.len() < MIN_FRAME_VALUES {
         return Err(Error::Parse(format!(
            "frame has {} values, need at least {MIN_FRAME_VALUES}",
            frames.len()
         )));
      }

      let from_color = frames[..3]
         .iter()
         .map(|&val| f64::from(val))
         .chain(iter::once(1.0))
         .collect::<Vec<f64>>();

      let to_color = frames[3..6]
         .iter()
         .map(|&val| f64::from(val))
         .chain(iter::once(1.0))
         .collect::<Vec<f64>>();

      let from_rotation = [0.0];
      let to_rotation = [Self::solve(f64::from(frames[6]), 60.0, 360.0, true)];

      let curves = frames[7..]
         .iter()
         .enumerate()
         .map(|(idx, &val)| Self::solve(f64::from(val), odd_coefficient(idx), 1.0, false))
         .collect::<Vec<f64>>();

      let cubic = Cubic::new(curves);
      let interpolation_factor = cubic.value(target_time);

      let color = interpolate(&from_color, &to_color, interpolation_factor)?
         .into_iter()
         .map(|val| val.clamp(0.0, 255.0))
         .collect::<Vec<_>>();

      let rotation = interpolate(&from_rotation, &to_rotation, interpolation_factor)?;
      let matrix = rotation_matrix(rotation[0]);

      let mut parts = Vec::with_capacity(9);

      for val in &color[..color.len() - 1] {
         parts.push(format!("{:x}", val.round() as i32));
      }

      for val in matrix {
         let rounded = (val * 100.0).round() / 100.0;
         let hex = float_to_hex(rounded.abs());

         if hex.starts_with('.') {
            parts.push(format!("0{}", hex.to_lowercase()));
         } else if hex.is_empty() {
            parts.push("0".to_owned());
         } else {
            parts.push(hex.to_lowercase());
         }
      }

      parts.push("0".to_owned());
      parts.push("0".to_owned());

      Ok(parts.join("").replace(['.', '-'], ""))
   }

   fn compute_animation_key(
      key_bytes: &[u8],
      html: &str,
      row_index: usize,
      key_bytes_indices: &[usize],
   ) -> Result<String, Error> {
      let row_selector = key_bytes
         .get(row_index)
         .ok_or_else(|| Error::Parse("key too short for row selection".into()))?;
      let row_index_value = usize::from(row_selector % ROW_INDEX_MODULUS);

      let frame_time = key_bytes_indices
         .iter()
         .filter_map(|&index| key_bytes.get(index))
         .map(|&byte| f64::from(byte % ROW_INDEX_MODULUS))
         .product::<f64>();

      let frame_time = js_round(frame_time / 10.0) * 10.0;

      let arr = Self::frame_data(key_bytes, html)?;

      let frame = arr
         .get(row_index_value)
         .ok_or_else(|| Error::Parse("row index out of bounds".into()))?;

      let target_time = frame_time / TOTAL_ANIMATION_TIME;
      Self::animate(frame, target_time)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn verification_key_extraction() {
      let html = r#"<html><head><meta name="twitter-site-verification" content="abc123xyz"/></head></html>"#;
      let key = ClientTransaction::verification_key(html).unwrap();
      assert_eq!(key, "abc123xyz");
   }

   #[test]
   fn verification_key_missing() {
      let html = "<html><head></head></html>";
      ClientTransaction::verification_key(html).unwrap_err();
   }

   #[test]
   fn ondemand_url_extraction() {
      let html = r#"something "ondemand.s": "abc123def" something"#;
      let url = ClientTransaction::extract_ondemand_url(html).unwrap();
      assert_eq!(
         url,
         "https://abs.twimg.com/responsive-web/client-web/ondemand.s.abc123defa.js"
      );
   }

   #[test]
   fn ondemand_url_single_quotes() {
      let html = "something 'ondemand.s': 'xyz789' something";
      let url = ClientTransaction::extract_ondemand_url(html).unwrap();
      assert_eq!(
         url,
         "https://abs.twimg.com/responsive-web/client-web/ondemand.s.xyz789a.js"
      );
   }

   #[test]
   fn ondemand_url_missing() {
      let html = "no ondemand here";
      ClientTransaction::extract_ondemand_url(html).unwrap_err();
   }

   #[test]
   fn parse_path_coordinates() {
      let path = "M0 0 0 0C10 20 30 40 50 60C70 80 90 100 110 120";
      let result = ClientTransaction::parse_path_to_coordinates(path);
      assert_eq!(result.len(), 2);
      assert!(!result[0].is_empty());
      assert!(!result[1].is_empty());
   }

   #[test]
   fn animate_insufficient_frames() {
      let frames = vec![1, 2, 3];
      ClientTransaction::animate(&frames, 0.5).unwrap_err();
   }

   #[test]
   fn indices_parsing() {
      let js = "foo(e[5], 16)bar(e[10], 16)padding";
      let (row_index, indices) = ClientTransaction::parse_indices(js).unwrap();
      assert_eq!(row_index, 5);
      assert_eq!(indices, vec![10]);
   }

   #[test]
   fn indices_missing() {
      let js = "no indices here";
      ClientTransaction::parse_indices(js).unwrap_err();
   }
}
