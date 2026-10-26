//! HTTP byte transport. The communicator does not interpret payloads.

use std::sync::{Arc, Mutex};

use tiny_http::{Method, Response, Server};

use super::{
    CLIENT_UPLOAD_PATH, COMMITTEE_FETCH_PATH, COMMITTEE_UPLOAD_PATH, Communicator, HTTP_ADDR,
    STATE_PATH,
};

impl Communicator {
    /// Serve `GET /state`, `POST /client-upload`, `POST /committee-upload`,
    /// and `POST /committee-fetch`.
    pub fn start_http_communicator(&self, state_bytes: Vec<u8>) {
        let messages = self.get_received_messages();
        let (committee_messages, committee_expected_size, committee_complete_callback) =
            self.get_committee_info();
        let committee_fetch_callback = self.get_committee_fetch_callback();
        std::thread::spawn(move || {
            if let Err(e) = serve(
                HTTP_ADDR,
                state_bytes,
                messages,
                committee_messages,
                committee_expected_size,
                committee_complete_callback,
                committee_fetch_callback,
            ) {
                eprintln!("HTTP communicator failed: {e}");
            }
        });
    }
}

fn serve(
    addr: &str,
    state_bytes: Vec<u8>,
    messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_expected_size: Option<usize>,
    committee_complete_callback: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
    committee_fetch_callback: Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>,
) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;
    println!("HTTP communicator listening on {addr}");
    serve_loop(
        server,
        state_bytes,
        messages,
        committee_messages,
        committee_expected_size,
        committee_complete_callback,
        committee_fetch_callback,
    );
    Ok(())
}

fn serve_loop(
    server: Server,
    state_bytes: Vec<u8>,
    messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_expected_size: Option<usize>,
    committee_complete_callback: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
    committee_fetch_callback: Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>,
) {
    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        match (method, url.as_str()) {
            (Method::Get, STATE_PATH) => {
                let _ = request.respond(Response::from_data(state_bytes.clone()));
            }
            (Method::Post, CLIENT_UPLOAD_PATH) => {
                let mut body = Vec::new();
                if request.as_reader().read_to_end(&mut body).is_ok() {
                    let mut queue = messages.lock().unwrap();
                    queue.push(body);
                    println!("Received client upload (total: {} messages)", queue.len());
                }
                let _ = request.respond(Response::empty(200));
            }
            (Method::Post, COMMITTEE_UPLOAD_PATH) => {
                let mut body = Vec::new();
                if request.as_reader().read_to_end(&mut body).is_ok() {
                    Communicator::push_committee_message(
                        &committee_messages,
                        committee_expected_size,
                        &committee_complete_callback,
                        body,
                    );
                }
                let _ = request.respond(Response::empty(200));
            }
            (Method::Post, COMMITTEE_FETCH_PATH) => {
                let mut body = Vec::new();
                let response = if request.as_reader().read_to_end(&mut body).is_ok() {
                    committee_fetch_callback
                        .as_ref()
                        .map(|callback| callback(&body))
                } else {
                    None
                };
                match response {
                    Some(bytes) => {
                        let _ = request.respond(Response::from_data(bytes));
                    }
                    None => {
                        let _ = request.respond(Response::empty(404));
                    }
                }
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
    use crate::communicator::native_transport::{get, post};

    fn listen(
        state: Vec<u8>,
        messages: Arc<Mutex<Vec<Vec<u8>>>>,
        expected_outputs: Option<usize>,
        on_complete: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
        on_fetch: Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>,
    ) -> String {
        let server = Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        std::thread::spawn(move || {
            serve_loop(
                server,
                state,
                messages,
                Arc::new(Mutex::new(Vec::new())),
                expected_outputs,
                on_complete,
                on_fetch,
            );
        });
        addr
    }

    #[test]
    fn request_state_round_trip() {
        let expected = vec![1u8, 2, 3, 4];
        let addr = listen(
            expected.clone(),
            Arc::new(Mutex::new(Vec::new())),
            None,
            None,
            None,
        );
        assert_eq!(get(&addr).unwrap(), expected);
    }

    #[test]
    fn send_client_upload_round_trip() {
        let expected = vec![1u8, 2, 3, 4];
        let messages = Arc::new(Mutex::new(Vec::new()));
        let addr = listen(Vec::new(), Arc::clone(&messages), None, None, None);
        post(&addr, CLIENT_UPLOAD_PATH, &expected).unwrap();
        assert_eq!(messages.lock().unwrap().as_slice(), &[expected]);
    }

    #[test]
    fn send_committee_output_round_trip() {
        let expected = b"committee-share".to_vec();
        let got = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&got);
        let addr = listen(
            Vec::new(),
            Arc::new(Mutex::new(Vec::new())),
            Some(1),
            Some(Arc::new(move |batch| *slot.lock().unwrap() = Some(batch))),
            None,
        );
        post(&addr, COMMITTEE_UPLOAD_PATH, &expected).unwrap();
        assert_eq!(got.lock().unwrap().clone().unwrap(), vec![expected]);
    }

    #[test]
    fn fetch_committee_inputs_round_trip() {
        let index = 3u16.to_le_bytes();
        let addr = listen(
            Vec::new(),
            Arc::new(Mutex::new(Vec::new())),
            None,
            None,
            Some(Arc::new(|body: &[u8]| body.to_vec())),
        );
        assert_eq!(post(&addr, COMMITTEE_FETCH_PATH, &index).unwrap(), index);
    }
}
