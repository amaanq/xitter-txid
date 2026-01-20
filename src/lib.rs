//! Generate X (Twitter) client transaction IDs.
//!
//! ```ignore
//! use xitter_txid::ClientTransaction;
//!
//! let client = ClientTransaction::fetch()?;
//! let id = client.generate_transaction_id("GET", "/i/api/1.1/jot/client_event.json");
//! ```
//!
//! To bring your own HTTP client:
//!
//! ```ignore
//! use xitter_txid::ClientTransaction;
//!
//! let html = your_client.get("https://x.com").text()?;
//! let js_url = ClientTransaction::extract_ondemand_url(&html)?;
//! let js = your_client.get(&js_url).text()?;
//! let client = ClientTransaction::new(&html, &js)?;
//! ```

mod cubic_curve;
mod error;
mod interpolate;
mod rotation;
mod transaction;
mod utils;

pub use error::Error;
pub use transaction::ClientTransaction;
