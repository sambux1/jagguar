use crate::protocols::Protocol;
use crate::protocols::server::Server;
use crate::protocols::client::Client;

pub struct Simulator<P: Protocol> {
	server: Option<P::Server>,
	clients: Vec<P::Client>,
	_marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
	pub fn new() -> Self {
		Self {
			server: None,
			clients: Vec::new(),
			_marker: core::marker::PhantomData,
		}
	}

	pub fn start_server(&mut self, server_parameters: <P::Server as Server>::SetupParameters) {
		// create the server
		let server = P::Server::new(server_parameters);
		
		// set the server in the simulator state
		self.server = Some(server);

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
}
