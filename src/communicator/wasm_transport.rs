//! Browser fetch transport for wasm client and committee roles.

use std::io;

use gloo_net::http::Request;

use super::{
    CLIENT_UPLOAD_PATH, COMMITTEE_FETCH_PATH, COMMITTEE_UPLOAD_PATH, STATE_PATH, Transport,
};

/// Party endpoint that talks to the aggregation server using browser `fetch`.
#[derive(Default)]
pub struct WasmTransport;

impl Transport for WasmTransport {
    async fn request_state(&self) -> io::Result<Vec<u8>> {
        get(STATE_PATH).await
    }

    async fn send_client_upload(&self, data: &[u8]) -> io::Result<()> {
        post(CLIENT_UPLOAD_PATH, data).await.map(|_| ())
    }

    async fn fetch_committee_inputs(&self, index: &[u8]) -> io::Result<Vec<u8>> {
        post(COMMITTEE_FETCH_PATH, index).await
    }

    async fn send_committee_output(&self, data: &[u8]) -> io::Result<()> {
        post(COMMITTEE_UPLOAD_PATH, data).await.map(|_| ())
    }
}

async fn get(path: &str) -> io::Result<Vec<u8>> {
    let response = Request::get(path)
        .send()
        .await
        .map_err(io::Error::other)?;
    response_bytes(response).await
}

async fn post(path: &str, data: &[u8]) -> io::Result<Vec<u8>> {
    let response = Request::post(path)
        .body(data.to_vec())
        .map_err(io::Error::other)?
        .send()
        .await
        .map_err(io::Error::other)?;
    response_bytes(response).await
}

async fn response_bytes(response: gloo_net::http::Response) -> io::Result<Vec<u8>> {
    if response.status() != 200 {
        return Err(io::Error::other(format!("HTTP {}", response.status())));
    }
    response.binary().await.map_err(io::Error::other)
}
