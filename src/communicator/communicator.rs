use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use socket2::{Socket, Domain, Type};

pub struct Communicator {
    port: u16,
    listener: Option<TcpListener>,
    shutdown: Option<Arc<AtomicBool>>,
}

impl Communicator {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            listener: None,
            shutdown: None,
        }
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
                    println!("Accepted connection from {:?}", addr);
                    drop(stream);
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

    pub fn connect_to_server(&self, server_port: u16) -> std::io::Result<TcpStream> {
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
}
