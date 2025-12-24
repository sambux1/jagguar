use crate::protocols::Protocol;
use crate::protocols::server::Server;
use crate::protocols::client::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc;

const STARTING_PORT: u16 = 10000;

pub struct Simulator<P: Protocol> {
	server_shutdown: Option<Arc<AtomicBool>>,
	server_state: Option<<P::Server as Server>::State>,
	clients: Vec<P::Client>,
	_marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
	pub fn new() -> Self {
		Self {
			server_shutdown: Some(Arc::new(AtomicBool::new(false))),
			server_state: None,
			clients: Vec::new(),
			_marker: core::marker::PhantomData,
		}
	}

	pub fn start_server(&mut self, server_parameters: <P::Server as Server>::SetupParameters)
	where
		P::Server: Send + 'static,
		<P::Server as Server>::SetupParameters: Send + 'static,
		<P::Server as Server>::State: Send,
	{
		let shutdown = Arc::clone(self.server_shutdown.as_ref().unwrap());

		// create a channel to send the server state back to the simulator
		let (state_sender, state_receiver) = mpsc::channel();

		// create server before running it in a thread
		let mut server = P::Server::new(server_parameters);

		// run the server in a thread
		std::thread::spawn(move || {
			server.setup_communicator(STARTING_PORT, shutdown, state_sender);
		});

		// wait for server to be ready and receive the state
		let state = state_receiver.recv().expect("Server failed to start");
		self.server_state = Some(state);
	}

	pub fn start_clients(&mut self, num_clients: usize)
	where
		<P::Server as Server>::State: Clone,
	{
		// create the clients
		for _ in 0..num_clients {
			let mut client = P::Client::new();
			client.set_server_state(self.server_state.as_ref().unwrap().clone());
			self.clients.push(client);
		}

		println!("{} clients started", self.clients.len());
	}

	pub fn teardown(&mut self) {
		// send the kill signal to the server through the shutdown flag
		self.server_shutdown.as_ref().unwrap().store(true, Ordering::Relaxed);
	}
}
