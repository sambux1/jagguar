use crate::protocols::Protocol;
use crate::protocols::server::Server;
use crate::protocols::client::Client;
use crate::protocols::committee::Committee;
use crate::crypto::prg::{default_prg, populate_random};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc;

const STARTING_PORT: u16 = 10000;
const INPUT_LEN: usize = 1024;

pub struct Simulator<P: Protocol> {
	server_shutdown: Option<Arc<AtomicBool>>,
	server_state: Option<<P::Server as Server>::State>,
	committee_port_offsets: Option<Vec<u16>>,
	/// Optional channel used by the server to send its final output back to the simulator
	server_output: Option<mpsc::Receiver<Vec<u32>>>,
	/// Channel used by clients to send their randomly chosen inputs back to the simulator
	client_input_channel: Option<mpsc::Receiver<Vec<u32>>>,
	/// Expected output: elementwise sum of all client inputs
	expected_output: Option<Vec<u64>>,
	_marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
	pub fn new() -> Self {
		Self {
			server_shutdown: Some(Arc::new(AtomicBool::new(false))),
			server_state: None,
			committee_port_offsets: None,
			server_output: None,
			client_input_channel: None,
			expected_output: None,
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

		// create a channel for the server to send its final output back
		let (output_sender, output_receiver) = mpsc::channel();
		self.server_output = Some(output_receiver);

		// create server before running it in a thread
		let mut server = P::Server::new(server_parameters);
		// allow the server to use the output channel if it chooses to
		server.set_output_channel(output_sender);
		self.committee_port_offsets = Some(server.get_committee_port_offsets());

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
		<P::Server as Server>::State: Clone + Send + 'static,
		P::Client: Send + 'static,
		P::Input: From<u32>,
	{
		// create a channel for clients to send their inputs back to the simulator
		let (input_sender, input_receiver) = mpsc::channel();
		self.client_input_channel = Some(input_receiver);

		let mut port = STARTING_PORT;
		// create the clients
		for _ in 0..num_clients {
			port += 1;

			let mut client = P::Client::new();
			client.set_server_state(self.server_state.as_ref().unwrap().clone());
			let sender = input_sender.clone();

			std::thread::spawn(move || {
				// generate a random input
				let mut rng = default_prg();
				let mut input = vec![0u32; INPUT_LEN];

				// generate random input, masked to the range [0, 2^20)
				populate_random(&mut input, &mut rng);
				input = input.iter().map(|x| x % (1 << 20) as u32).collect();
				
				// send the input to the simulator (non-blocking for unbounded channel)
				let _ = sender.send(input.clone());
				
				let input: Vec<P::Input> = input.into_iter().map(|x| x.into()).collect();

				// set the client's input
				client.set_input(input);

				// encrypt the input
				client.encrypt_input();

				// send the input to the server
				// the port is automatically passed by value, so this is thread-safe
				client.send_input(port);
			});
		}

		println!("{} clients started", num_clients);
	}

	pub fn start_committee(&mut self)
	where
		<P::Server as Server>::State: Clone + Send + 'static,
		P::Committee: Send + 'static,
	{
		// make sure server state and port offsets are set
		assert!(self.server_state.is_some(), "Server state is not set");
		assert!(self.committee_port_offsets.is_some(), "Committee port offsets are not set");

		let server_state = self.server_state.as_ref().unwrap();
		let port_offsets = self.committee_port_offsets.as_ref().unwrap();

		for (_, port_offset) in port_offsets.iter().enumerate() {
			let port = STARTING_PORT + *port_offset;

			// make a new committee member and set its server state
			let mut committee_member = P::Committee::new(port);
			committee_member.set_server_state(server_state.clone());

			// run the committee member in a thread
			std::thread::spawn(move || {
				committee_member.retrieve_inputs();
				committee_member.aggregate();
				committee_member.send_output();
			});
		}
	}

	pub fn collect_client_inputs(&mut self, expected_count: usize) -> Vec<Vec<u32>> {
		let mut inputs = Vec::new();
		if let Some(ref receiver) = self.client_input_channel {
			// loop over all received messages until we have all expected inputs
			// or until the channel is disconnected
			while inputs.len() < expected_count {
				match receiver.recv_timeout(std::time::Duration::from_millis(100)) {
					Ok(input) => {
						inputs.push(input);
					}
					Err(mpsc::RecvTimeoutError::Timeout) => {
						// continue waiting if we haven't received all inputs yet
						continue;
					}
					Err(mpsc::RecvTimeoutError::Disconnected) => {
						// channel disconnected, break and return what we have
						break;
					}
				}
			}
		}
		
		// compute elementwise sum over all client inputs
		let sum: Vec<u64> = if inputs.is_empty() {
			Vec::new()
		} else {
			let len = inputs[0].len();
			(0..len)
				.map(|i| {
					inputs
						.iter()
						.map(|input| input[i] as u64)
						.sum()
				})
				.collect()
		};
		
		// store the expected output
		self.expected_output = Some(sum);
		
		inputs
	}

	pub fn output(&mut self) {
		match self.server_output {
			Some(ref receiver) => match receiver.try_recv() {
				Ok(output) => {
					// compare with expected output
					if let Some(ref expected) = self.expected_output {
						if output.len() == expected.len() {
							let matches = output
								.iter()
								.zip(expected.iter())
								.all(|(&actual, &expected)| actual as u64 == expected);
							
							if matches {
								println!("Output matches expected sum!");
							} else {
								println!("Output does NOT match expected sum!");
							}
						} else {
							println!("Output length mismatch! Expected {} values, got {}", expected.len(), output.len());
						}
					} else {
						println!("Warning: No expected output available for comparison");
					}
				}
				Err(mpsc::TryRecvError::Empty) => {
					println!("Server output: <no output available yet>");
				}
				Err(mpsc::TryRecvError::Disconnected) => {
					println!("Server output channel disconnected");
				}
			},
			None => {
				println!("Server output receiver not configured");
			}
		}
	}

	pub fn teardown(&mut self) {
		// send the kill signal to the server through the shutdown flag
		self.server_shutdown.as_ref().unwrap().store(true, Ordering::Relaxed);
	}
}
