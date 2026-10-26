//! Blocking HTTP transport for native client and committee binaries.

use std::io::{Read, Write};
use std::net::TcpStream;

use super::{
    CLIENT_UPLOAD_PATH, COMMITTEE_FETCH_PATH, COMMITTEE_UPLOAD_PATH, HTTP_ADDR, STATE_PATH,
};

/// Party endpoint that talks to the aggregation server over blocking HTTP.
///
/// `id` is this party's identity, used by the committee to derive its
/// index relative to the server's port.
pub struct NativeTransport {
    id: u16,
}

impl NativeTransport {
    /// Native transport for party with identity `id`.
    pub fn new(id: u16) -> Self {
        Self { id }
    }

    /// Party identity.
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Request public state from the aggregation server.
    pub fn request_state(&self) -> std::io::Result<Vec<u8>> {
        get(HTTP_ADDR)
    }

    /// Send a client upload to the aggregation server.
    pub fn send_client_upload(&self, data: &[u8]) -> std::io::Result<()> {
        post(HTTP_ADDR, CLIENT_UPLOAD_PATH, data).map(|_| ())
    }

    /// Send a committee output to the aggregation server.
    pub fn send_committee_output(&self, data: &[u8]) -> std::io::Result<()> {
        post(HTTP_ADDR, COMMITTEE_UPLOAD_PATH, data).map(|_| ())
    }

    /// Fetch one committee member's inputs from the aggregation server.
    pub fn fetch_committee_inputs(&self, index: &[u8]) -> std::io::Result<Vec<u8>> {
        post(HTTP_ADDR, COMMITTEE_FETCH_PATH, index)
    }
}

pub(crate) fn request(
    addr: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr)?;
    write!(
        stream,
        "{method} {path} HTTP/1.0\r\n\
         Host: {addr}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("no HTTP header terminator"))?;
    let status = buf[..split]
        .split(|&b| b == b'\n')
        .next()
        .and_then(|line| std::str::from_utf8(line).ok()?.split_whitespace().nth(1))
        .ok_or_else(|| std::io::Error::other("no HTTP status"))?;
    if status != "200" {
        return Err(std::io::Error::other(format!("HTTP {status}")));
    }
    Ok(buf[split + 4..].to_vec())
}

pub(crate) fn get(addr: &str) -> std::io::Result<Vec<u8>> {
    request(addr, "GET", STATE_PATH, &[])
}

pub(crate) fn post(addr: &str, path: &str, data: &[u8]) -> std::io::Result<Vec<u8>> {
    request(addr, "POST", path, data)
}
