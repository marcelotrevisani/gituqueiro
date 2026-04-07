#[allow(dead_code)]
pub mod client;
#[allow(dead_code)]
pub mod oauth;
#[allow(dead_code)]
pub mod types;

/// Build a `reqwest::Client` that respects system proxy settings (via
/// `HTTPS_PROXY` / `HTTP_PROXY` env vars) and trusts native OS root
/// certificates, including corporate CAs installed by tools like Zscaler.
pub fn build_http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .tls_built_in_native_certs(true)
        .build()
        .expect("Failed to build HTTP client")
}
