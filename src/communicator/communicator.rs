use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::thread;
use socket2::{Socket, Domain, Type};

pub struct Communicator {
    port: u16,
    listener: Option<TcpListener>,
    shutdown: Option<Arc<AtomicBool>>,
    received_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    signal_callback: Option<Arc<dyn Fn(TcpStream, u16) + Send + Sync>>,
	committee_expected_size: Option<usize>,
	committee_complete_callback: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
}

impl Communicator {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            listener: None,
            shutdown: None,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            committee_messages: Arc::new(Mutex::new(Vec::new())),
            signal_callback: None,
			committee_expected_size: None,
			committee_complete_callback: None,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn set_signal_callback<F>(&mut self, callback: F)
    where
        F: Fn(TcpStream, u16) + Send + Sync + 'static,
    {
        self.signal_callback = Some(Arc::new(callback));
    }

	pub fn set_committee_expected_size(&mut self, size: usize) {
		self.committee_expected_size = Some(size);
	}

	pub fn set_committee_complete_callback<F>(&mut self, callback: F)
	where
		F: Fn(Vec<Vec<u8>>) + Send + Sync + 'static,
	{
		self.committee_complete_callback = Some(Arc::new(callback));
	}

    pub fn set_shutdown_flag(&mut self, shutdown: Arc<AtomicBool>) {
        self.shutdown = Some(shutdown);
    }

    pub fn start_server(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port))?;
        self.listener = Some(listener);
        Ok(())
    }

    pub fn listen_loop(&self) -> std::io::Result<()> {
        let _ = self.listener.as_ref().unwrap().set_nonblocking(true);
        loop {
            if let Some(ref shutdown) = self.shutdown {
                if shutdown.load(Ordering::Relaxed) {
                    println!("Server shutting down gracefully");
                    break;
                }
            }
            match self.listener.as_ref().unwrap().accept() {
                Ok((stream, addr)) => {
                    let messages = Arc::clone(&self.received_messages);
                    let committee_messages = Arc::clone(&self.committee_messages);
                    let callback = self.signal_callback.clone();
					let committee_expected_size = self.committee_expected_size;
					let committee_complete_callback = self.committee_complete_callback.clone();
                    thread::spawn(move || {
						Self::handle_connection(
							stream,
							addr,
							messages,
							committee_messages,
							callback,
							committee_expected_size,
							committee_complete_callback,
						);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                }
            }
        }
        Ok(())
    }

    fn handle_connection(
        mut stream: TcpStream,
        addr: SocketAddr,
        messages: Arc<Mutex<Vec<Vec<u8>>>>,
        committee_messages: Arc<Mutex<Vec<Vec<u8>>>>,
        callback: Option<Arc<dyn Fn(TcpStream, u16) + Send + Sync>>,
		committee_expected_size: Option<usize>,
		committee_complete_callback: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
    ) {
        // read exactly 6 bytes to check if it's a signal
        let mut signal_buffer = vec![0u8; 6];
        match stream.read_exact(&mut signal_buffer) {
            Ok(_) => {
                if signal_buffer.starts_with(b"signal") {
                    // it's a signal - call the callback with the stream
                    println!("Signal received from {:?}", addr);
                    if let Some(ref cb) = callback {
                        cb(stream, addr.port());
                    }
                    return;
                }
                // not a signal - read the rest and combine with what we already read
                let mut rest = Vec::new();
                let _ = stream.read_to_end(&mut rest);
                signal_buffer.extend_from_slice(&rest);
                
                // check if it's a committee message
                if signal_buffer.starts_with(b"committee") {
					let mut committee_queue = committee_messages.lock().unwrap();
					committee_queue.push(signal_buffer);
					let current_len = committee_queue.len();
					println!("Committee message queued (total: {} committee messages)", current_len);

					// If we have the expected number of committee messages, trigger the callback
					if let Some(expected) = committee_expected_size {
						if current_len == expected {
							// Take a snapshot and clear the queue before invoking the callback
							let batch = committee_queue.clone();
							committee_queue.clear();
							drop(committee_queue);
							if let Some(ref cb) = committee_complete_callback {
								cb(batch);
							}
						}
					}
                } else {
                    // regular client message
                    let mut messages = messages.lock().unwrap();
                    messages.push(signal_buffer);
                    println!("Received message from {:?} (total: {} messages)", addr, messages.len());
                }
            }
            Err(e) => {
                eprintln!("Failed to read from {:?}: {}", addr, e);
            }
        }
    }

    pub fn get_received_messages(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.received_messages)
    }

    pub fn get_committee_messages(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.committee_messages)
    }

    fn connect_to_server(&self, server_port: u16) -> std::io::Result<TcpStream> {
        // create a socket and bind to our local port
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_reuse_address(true)?;
        let local_addr: SocketAddr = format!("127.0.0.1:{}", self.port).parse().unwrap();
        socket.bind(&local_addr.into())?;

        // connect to the server's port
        let server_addr: SocketAddr = format!("127.0.0.1:{}", server_port).parse().unwrap();
        socket.connect(&server_addr.into())?;

        // convert to TcpStream
        let stream = TcpStream::from(socket);
        Ok(stream)
    }

    pub fn send_to_server(&self, server_port: u16, data: &[u8]) -> std::io::Result<()> {
        let mut stream = self.connect_to_server(server_port)?;
        stream.write_all(data)?;
        Ok(())
    }

    pub fn signal_server(&self, server_port: u16) -> std::io::Result<TcpStream> {
        let mut stream = self.connect_to_server(server_port)?;
        stream.write_all(b"signal")?;
        Ok(stream)
    }

    pub fn receive_from_server(&self, server_port: u16) -> std::io::Result<Vec<u8>> {
        let mut stream = self.signal_server(server_port)?;
        let mut buffer = Vec::new();
        stream.read_to_end(&mut buffer)?;
        println!("Received {} bytes from server", buffer.len());
        Ok(buffer)
    }

    pub fn send_on_stream(mut stream: TcpStream, inputs: Vec<Vec<u8>>) -> std::io::Result<()> {
        // serialize all inputs together
        let mut data = Vec::new();
        for input in &inputs {
            // Prefix each input with its length for parsing
            let len = input.len() as u32;
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(input);
        }
        
        // Send the data through the TCP stream
        stream.write_all(&data)?;
        stream.flush()?;
        // Shutdown the write side so the reader knows there's no more data
        stream.shutdown(std::net::Shutdown::Write)?;
        println!("Sent {} inputs to committee member", inputs.len());
        Ok(())
    }
}
