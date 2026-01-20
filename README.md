# xitter-txid

Generate X (Twitter) client transaction IDs in Rust.

X's API requires a `x-client-transaction-id` header for authenticated requests. This library extracts the cryptographic
material from X's homepage and JavaScript files to generate valid transaction IDs.

## Installation

```toml
[dependencies]
xitter-txid = "0.1"
```

To use your own HTTP client and skip the built-in one:

```toml
[dependencies]
xitter-txid = { version = "0.1", default-features = false }
```

## Usage

```rust
use xitter_txid::ClientTransaction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientTransaction::fetch()?;

    let id = client.generate_transaction_id("POST", "/i/api/1.1/jot/client_event.json");
    println!("{id}");

    Ok(())
}
```

### Bring your own HTTP client

If you need custom headers, proxies, or async support:

```rust
use xitter_txid::ClientTransaction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = your_client.get("https://x.com")?.text()?;
    let js_url = ClientTransaction::extract_ondemand_url(&html)?;
    let js = your_client.get(&js_url)?.text()?;

    let client = ClientTransaction::new(&html, &js)?;
    let id = client.generate_transaction_id("GET", "/i/api/graphql/abc123/UserByScreenName");

    Ok(())
}
```

## License

MIT
