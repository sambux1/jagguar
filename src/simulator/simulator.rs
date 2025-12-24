use crate::protocols::Protocol;
use crate::protocols::server::Server;
use crate::protocols::client::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const STARTING_PORT: u16 = 10000;

pub struct Simulator<P: Protocol> {
	server_shutdown: Option<Arc<AtomicBool>>,
	clients: Vec<P::Client>,
	_marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
	pub fn new() -> Self {
		Self {
			server_shutdown: Some(Arc::new(AtomicBool::new(false))),
			clients: Vec::new(),
			_marker: core::marker::PhantomData,
		}
	}

	pub fn start_server(&mut self, server_parameters: <P::Server as Server>::SetupParameters)
	where
		P::Server: Send + 'static,
		<P::Server as Server>::SetupParameters: Send + 'static,
	{
		let shutdown = Arc::clone(self.server_shutdown.as_ref().unwrap());

		std::thread::spawn(move || {
			let mut server = P::Server::new(server_parameters);
			server.setup_communicator(STARTING_PORT, shutdown);
		});

		println!("Server started");
	}

	pub fn start_clients(&mut self, num_clients: usize) {
		// create the clients
		for _ in 0..num_clients {
			let client = P::Client::new();
			self.clients.push(client);
		}

		println!("{} clients started", self.clients.len());
	}

	pub fn teardown(&mut self) {
		// send the kill signal to the server through the shutdown flag
		self.server_shutdown.as_ref().unwrap().store(true, Ordering::Relaxed);
	}
}
