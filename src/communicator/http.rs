//! HTTP byte transport. The communicator does not interpret payloads.

use std::io::{Read, Write};
use std::net::TcpStream;

use tiny_http::{Response, Server};

use super::Communicator;

const HTTP_ADDR: &str = "127.0.0.1:8080";

impl Communicator {
    /// Serve `state_bytes` on GET in a background thread.
    pub fn start_http_communicator(&self, state_bytes: Vec<u8>) {
        std::thread::spawn(move || {
            if let Err(e) = serve(HTTP_ADDR, state_bytes) {
                eprintln!("HTTP stub failed: {e}");
            }
        });
    }

    /// Request public state from the aggregation server.
    pub fn request_state(&self) -> std::io::Result<Vec<u8>> {
        get(HTTP_ADDR)
    }
}

fn get(addr: &str) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(addr)?;
    write!(
        stream,
        "GET / HTTP/1.0\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let split = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::other("no HTTP header terminator"))?;
    Ok(buf[split + 4..].to_vec())
}

fn serve(addr: &str, state_bytes: Vec<u8>) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;
    println!("HTTP communicator listening on {addr}");
    serve_loop(server, state_bytes);
    Ok(())
}

fn serve_loop(server: Server, state_bytes: Vec<u8>) {
    for request in server.incoming_requests() {
        let _ = request.respond(Response::from_data(state_bytes.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_state_round_trip() {
        let want = vec![1u8, 2, 3, 4];
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let body = want.clone();
        std::thread::spawn(move || serve_loop(server, body));
        let got = get(&addr).unwrap();
        assert_eq!(got, want);
    }
}
