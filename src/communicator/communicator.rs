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
    signal_callback: Option<Arc<dyn Fn(SocketAddr) + Send + Sync>>,
}

impl Communicator {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            listener: None,
            shutdown: None,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            signal_callback: None,
        }
    }

    pub fn set_signal_callback<F>(&mut self, callback: F)
    where
        F: Fn(SocketAddr) + Send + Sync + 'static,
    {
        self.signal_callback = Some(Arc::new(callback));
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
                    let callback = self.signal_callback.clone();
                    thread::spawn(move || {
                        Self::handle_connection(stream, addr, messages, callback);
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
        callback: Option<Arc<dyn Fn(SocketAddr) + Send + Sync>>,
    ) {
        let mut buffer = Vec::new();
        match stream.read_to_end(&mut buffer) {
            Ok(_) => {
                // check if this is a signal (starts with "signal")
                if buffer.starts_with(b"signal") {
                    Self::handle_signal(addr, callback);
                } else {
                    let mut messages = messages.lock().unwrap();
                    messages.push(buffer);
                    println!("Received message from {:?} (total: {} messages)", addr, messages.len());
                }
            }
            Err(e) => {
                eprintln!("Failed to read from {:?}: {}", addr, e);
            }
        }
    }

    fn handle_signal(addr: SocketAddr, callback: Option<Arc<dyn Fn(SocketAddr) + Send + Sync>>) {
        println!("Signal handler called, received from {:?}", addr);
        if let Some(ref cb) = callback {
            cb(addr);
        }
    }

    pub fn get_received_messages(&self) -> Vec<Vec<u8>> {
        let messages = self.received_messages.lock().unwrap();
        messages.clone()
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

    pub fn signal_server(&self, server_port: u16) -> std::io::Result<()> {
        let mut stream = self.connect_to_server(server_port)?;
        stream.write_all(b"signal")?;
        Ok(())
    }
}
