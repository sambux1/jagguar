use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
}
