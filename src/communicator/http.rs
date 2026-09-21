//! HTTP byte transport. The communicator does not interpret payloads.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use tiny_http::{Method, Response, Server};

use super::Communicator;

const HTTP_ADDR: &str = "127.0.0.1:8080";
const STATE_PATH: &str = "/state";
const UPLOAD_PATH: &str = "/upload";

impl Communicator {
    /// Serve state on `GET /state` and queue `POST /upload` bodies as client uploads.
    pub fn start_http_communicator(&self, state_bytes: Vec<u8>) {
        let messages = self.get_received_messages();
        std::thread::spawn(move || {
            if let Err(e) = serve(HTTP_ADDR, state_bytes, messages) {
                eprintln!("HTTP communicator failed: {e}");
            }
        });
    }

    /// Request public state from the aggregation server.
    pub fn request_state(&self) -> std::io::Result<Vec<u8>> {
        get(HTTP_ADDR)
    }

    /// Send a client upload to the aggregation server.
    pub fn send_upload(&self, data: &[u8]) -> std::io::Result<()> {
        post(HTTP_ADDR, data).map(|_| ())
    }
}

fn request(addr: &str, method: &str, path: &str, body: &[u8]) -> std::io::Result<Vec<u8>> {
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
        .and_then(|line| {
            std::str::from_utf8(line)
                .ok()?
                .split_whitespace()
                .nth(1)
        })
        .ok_or_else(|| std::io::Error::other("no HTTP status"))?;
    if status != "200" {
        return Err(std::io::Error::other(format!("HTTP {status}")));
    }
    Ok(buf[split + 4..].to_vec())
}

fn get(addr: &str) -> std::io::Result<Vec<u8>> {
    request(addr, "GET", STATE_PATH, &[])
}

fn post(addr: &str, data: &[u8]) -> std::io::Result<Vec<u8>> {
    request(addr, "POST", UPLOAD_PATH, data)
}

fn serve(
    addr: &str,
    state_bytes: Vec<u8>,
    messages: Arc<Mutex<Vec<Vec<u8>>>>,
) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;
    println!("HTTP communicator listening on {addr}");
    serve_loop(server, state_bytes, messages);
    Ok(())
}

fn serve_loop(server: Server, state_bytes: Vec<u8>, messages: Arc<Mutex<Vec<Vec<u8>>>>) {
    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        match (method, url.as_str()) {
            (Method::Get, STATE_PATH) => {
                let _ = request.respond(Response::from_data(state_bytes.clone()));
            }
            (Method::Post, UPLOAD_PATH) => {
                let mut body = Vec::new();
                if request.as_reader().read_to_end(&mut body).is_ok() {
                    let mut queue = messages.lock().unwrap();
                    queue.push(body);
                    println!(
                        "Received client upload (total: {} messages)",
                        queue.len()
                    );
                }
                let _ = request.respond(Response::empty(200));
            }
            _ => {
                let _ = request.respond(Response::empty(404));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_state_round_trip() {
        let expected = vec![1u8, 2, 3, 4];
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let body = expected.clone();
        let messages = Arc::new(Mutex::new(Vec::new()));
        std::thread::spawn(move || serve_loop(server, body, messages));
        let got = get(&addr).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn send_upload_round_trip() {
        let expected = vec![1u8, 2, 3, 4];
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let messages = Arc::new(Mutex::new(Vec::new()));
        let queue = Arc::clone(&messages);
        std::thread::spawn(move || serve_loop(server, Vec::new(), queue));
        post(&addr, &expected).unwrap();
        assert_eq!(messages.lock().unwrap().as_slice(), &[expected]);
    }
}
