//! Error types.

use std::{
   error::Error as StdError,
   fmt,
};

#[derive(Debug)]
pub enum Error {
   /// Interpolation arrays have different lengths.
   MismatchedArguments,
   /// Failed to parse HTML or JavaScript.
   Parse(String),
   /// Required key or attribute not found.
   MissingKey(String),
   /// Base64 decoding failed.
   Base64(data_encoding::DecodeError),
   /// HTTP request failed.
   #[cfg(feature = "fetch")]
   Http(minreq::Error),
   /// HTTP response returned non-200 status.
   #[cfg(feature = "fetch")]
   HttpStatus(i32, &'static str),
}

impl fmt::Display for Error {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      match *self {
         Self::MismatchedArguments => {
            write!(f, "interpolation arrays have different lengths")
         },
         Self::Parse(ref msg) => write!(f, "parse error: {msg}"),
         Self::MissingKey(ref key) => write!(f, "missing required key: {key}"),
         Self::Base64(ref err) => write!(f, "base64 decode error: {err}"),
         #[cfg(feature = "fetch")]
         Self::Http(ref err) => write!(f, "HTTP error: {err}"),
         #[cfg(feature = "fetch")]
         Self::HttpStatus(code, url) => write!(f, "{url} returned HTTP {code}"),
      }
   }
}

impl StdError for Error {
   fn source(&self) -> Option<&(dyn StdError + 'static)> {
      match *self {
         Self::Base64(ref err) => Some(err),
         Self::MismatchedArguments | Self::Parse(_) | Self::MissingKey(_) => None,
         #[cfg(feature = "fetch")]
         Self::Http(ref err) => Some(err),
         #[cfg(feature = "fetch")]
         Self::HttpStatus(..) => None,
      }
   }
}

impl From<data_encoding::DecodeError> for Error {
   fn from(err: data_encoding::DecodeError) -> Self {
      Self::Base64(err)
   }
}

#[cfg(feature = "fetch")]
impl From<minreq::Error> for Error {
   fn from(err: minreq::Error) -> Self {
      Self::Http(err)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn error_display() {
      let err = Error::MismatchedArguments;
      assert!(err.to_string().contains("different lengths"));

      let err = Error::Parse("test".into());
      assert!(err.to_string().contains("test"));

      let err = Error::MissingKey("verification".into());
      assert!(err.to_string().contains("verification"));
   }

   #[test]
   fn error_source() {
      let err = Error::MismatchedArguments;
      assert!(err.source().is_none());

      let err = Error::Parse("test".into());
      assert!(err.source().is_none());
   }
}
