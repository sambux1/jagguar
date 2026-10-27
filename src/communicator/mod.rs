//! Localhost HTTP transport for the simulator.
//!
//! HTTP carries `GET /state`, `POST /client-upload`, `POST /committee-upload`,
//! and `POST /committee-fetch`. Payloads are opaque bytes. There is no
//! authentication.
//!
//! [`wire`] holds party IDs and the codec used by typed messages.

pub mod wire;

#[cfg(feature = "native")]
pub mod communicator;
#[cfg(feature = "native")]
pub mod http;
#[cfg(feature = "native")]
pub mod native_transport;
#[cfg(target_arch = "wasm32")]
pub mod wasm_transport;

#[cfg(feature = "native")]
pub use communicator::Communicator;
#[cfg(feature = "native")]
pub use native_transport::NativeTransport;
#[cfg(target_arch = "wasm32")]
pub use wasm_transport::WasmTransport;

#[cfg(feature = "native")]
pub type DefaultTransport = NativeTransport;
#[cfg(target_arch = "wasm32")]
pub type DefaultTransport = WasmTransport;

#[allow(async_fn_in_trait)]
pub trait Transport {
    async fn request_state(&self) -> std::io::Result<Vec<u8>>;
    async fn send_client_upload(&self, data: &[u8]) -> std::io::Result<()>;
    async fn fetch_committee_inputs(&self, index: &[u8]) -> std::io::Result<Vec<u8>>;
    async fn send_committee_output(&self, data: &[u8]) -> std::io::Result<()>;
}

/// Localhost address the server binds and the native transport connects to.
#[cfg(feature = "native")]
pub(crate) const HTTP_ADDR: &str = "127.0.0.1:8080";

pub(crate) const STATE_PATH: &str = "/state";
pub(crate) const CLIENT_UPLOAD_PATH: &str = "/client-upload";
pub(crate) const COMMITTEE_UPLOAD_PATH: &str = "/committee-upload";
pub(crate) const COMMITTEE_FETCH_PATH: &str = "/committee-fetch";
