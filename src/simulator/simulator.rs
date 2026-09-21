//! Thread-per-party localhost run of a [`crate::protocols::Protocol`].

use crate::crypto::prg::{default_prg, populate_random};
use crate::protocols::Protocol;
use crate::protocols::client::Client;
use crate::protocols::committee::Committee;
use crate::protocols::server::Server;
use crate::simulator::port_pool::PortPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

/// First port in the simulator pool (server is typically this port).
const STARTING_PORT: u16 = 10000;
/// Size of the allocatable port range.
const DEFAULT_MAX_PORTS: usize = 1000;
/// Length of each randomly generated client input.
const INPUT_LEN: usize = 1024;

/// Localhost run of protocol `P`.
pub struct Simulator<P: Protocol> {
    /// Set by [`Self::teardown`] to stop the server listen loop.
    server_shutdown: Option<Arc<AtomicBool>>,
    /// Committee port offsets from the server, set by [`Self::start_server`].
    committee_port_offsets: Option<Vec<u16>>,
    /// Optional channel used by the server to send its final output back to the simulator
    server_output: Option<mpsc::Receiver<Vec<u32>>>,
    /// Channel used by clients to send their randomly chosen inputs back to the simulator
    client_input_channel: Option<mpsc::Receiver<Vec<u32>>>,
    /// Expected output: elementwise sum of all client inputs
    expected_output: Option<Vec<u64>>,
    /// Port pool for managing port allocation and status
    port_pool: PortPool,
    _marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
    /// Empty simulator with a port pool starting at 10000.
    pub fn new() -> Self {
        Self {
            server_shutdown: Some(Arc::new(AtomicBool::new(false))),
            committee_port_offsets: None,
            server_output: None,
            client_input_channel: None,
            expected_output: None,
            port_pool: PortPool::new(STARTING_PORT, DEFAULT_MAX_PORTS),
            _marker: core::marker::PhantomData,
        }
    }

    /// Spawn the server thread and wait until HTTP `request_state` succeeds.
    ///
    /// # Panics
    ///
    /// Panics if no port is available or the server never becomes reachable.
    pub fn start_server(&mut self, server_parameters: <P::Server as Server>::SetupParameters)
    where
        P::Server: Send + 'static,
        <P::Server as Server>::SetupParameters: Send + 'static,
    {
        let shutdown = Arc::clone(self.server_shutdown.as_ref().unwrap());

        // create a channel for the server to send its final output back
        let (output_sender, output_receiver) = mpsc::channel();
        self.server_output = Some(output_receiver);

        // create server before running it in a thread
        let mut server = P::Server::new(server_parameters);
        // allow the server to use the output channel if it chooses to
        server.set_output_channel(output_sender);
        self.committee_port_offsets = Some(server.get_committee_port_offsets());

        // run the server in a thread
        let port = self.port_pool.allocate_port().unwrap();
        std::thread::spawn(move || {
            server.setup_communicator(port, shutdown);
        });

        loop {
            let mut probe = P::Client::new();
            if probe.request_state().is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Spawn `num_clients` threads. Each samples a random 1024-vector
    /// in `[0, 2^20)`, reports it on the input channel, encrypts, and sends.
    ///
    /// # Panics
    ///
    /// Panics if a client cannot request state.
    pub fn start_clients(&mut self, num_clients: usize)
    where
        P::Client: Send + 'static,
        P::Input: From<u32>,
    {
        // create a channel for clients to send their inputs back to the simulator
        let (input_sender, input_receiver) = mpsc::channel();
        self.client_input_channel = Some(input_receiver);

        // create the clients
        for _ in 0..num_clients {
            let mut client = P::Client::new();
            let sender = input_sender.clone();

            std::thread::spawn(move || {
                client.request_state().expect("request state");

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
                client.send_input();
            });
        }

        println!("{} clients started", num_clients);
    }

    /// Spawn one committee thread per offset. Ports are
    /// `10000 + offset`, not taken from the pool.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::start_server`] has not run.
    pub fn start_committee(&mut self)
    where
        P::Committee: Send + 'static,
    {
        assert!(
            self.committee_port_offsets.is_some(),
            "Committee port offsets are not set"
        );

        let port_offsets = self.committee_port_offsets.as_ref().unwrap();

        for (_, port_offset) in port_offsets.iter().enumerate() {
            let port = STARTING_PORT + *port_offset;

            let mut committee_member = P::Committee::new(port);

            // run the committee member in a thread
            std::thread::spawn(move || {
                committee_member.request_state().expect("request state");
                committee_member.retrieve_inputs();
                committee_member.aggregate();
                committee_member.send_output();
            });
        }

        println!("{} committee members started", port_offsets.len());
    }

    /// Block until `expected_count` client inputs arrive (or the channel
    /// disconnects) and store their elementwise sum as the expected output.
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
                .map(|i| inputs.iter().map(|input| input[i] as u64).sum())
                .collect()
        };

        // store the expected output
        self.expected_output = Some(sum);

        inputs
    }

    /// Non-blocking: print whether the server output matches the expected sum.
    ///
    /// Prints a placeholder if the server has not sent yet. Call
    /// [`Self::collect_client_inputs`] first to populate the expected sum.
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
                            println!(
                                "Output length mismatch! Expected {} values, got {}",
                                expected.len(),
                                output.len()
                            );
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

    /// Set the server shutdown flag. Does not join party threads.
    pub fn teardown(&mut self) {
        // send the kill signal to the server through the shutdown flag
        self.server_shutdown
            .as_ref()
            .unwrap()
            .store(true, Ordering::Relaxed);
    }
}
