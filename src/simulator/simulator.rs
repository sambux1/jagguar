//! Thread-per-party localhost run of a [`crate::protocols::Protocol`].

use crate::crypto::prg::{default_prg, populate_random, populate_random_bytes};
use crate::protocols::Protocol;
use crate::protocols::client::Client;
use crate::protocols::server::Server;
use crate::simulator::port_pool::PortPool;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};

unsafe extern "C" {
    fn dup2(oldfd: i32, newfd: i32) -> i32;
}

/// File descriptor the server process writes the decoded aggregate to.
const SERVER_OUTPUT_FD: i32 = 3;

/// First port in the simulator pool (server is typically this port).
const STARTING_PORT: u16 = 10000;
/// Size of the allocatable port range.
const DEFAULT_MAX_PORTS: usize = 1000;
/// Length of each randomly generated client input.
const INPUT_LEN: usize = 1024;

/// Localhost run of protocol `P`.
pub struct Simulator<P: Protocol> {
    /// Committee port offsets from the server, set by [`Self::start_server`].
    committee_port_offsets: Option<Vec<u16>>,
    /// Expected output: elementwise sum of all client inputs
    expected_output: Option<Vec<u64>>,
    /// Seeds passed to client processes.
    client_seeds: Vec<[u8; 32]>,
    /// Read end of the pipe carrying the decoded aggregate.
    server_output_pipe: Option<UnixStream>,
    /// Port pool for managing port allocation and status
    port_pool: PortPool,
    /// Child process running the aggregation server, if started.
    server_process: Option<Child>,
    /// Child processes running clients.
    client_processes: Vec<Child>,
    /// Child processes running committee members.
    committee_processes: Vec<Child>,
    _marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
    /// Empty simulator with a port pool starting at 10000.
    pub fn new() -> Self {
        Self {
            committee_port_offsets: None,
            expected_output: None,
            client_seeds: Vec::new(),
            server_output_pipe: None,
            port_pool: PortPool::new(STARTING_PORT, DEFAULT_MAX_PORTS),
            server_process: None,
            client_processes: Vec::new(),
            committee_processes: Vec::new(),
            _marker: core::marker::PhantomData,
        }
    }

    /// Start the aggregation server as a child process and wait until it
    /// is reachable.
    ///
    /// # Panics
    ///
    /// Panics if no port is available or the server binary cannot be spawned.
    pub fn start_server(&mut self, server_parameters: <P::Server as Server>::SetupParameters) {
        let _server_parameters = server_parameters;
        let port = self.port_pool.allocate_port().unwrap();

        let mut server = std::env::current_exe().unwrap();
        server.set_file_name("server");
        let (reader, writer) = UnixStream::pair().unwrap();
        // Spawn the server as a separate process.
        let mut cmd = Command::new(&server);
        cmd.arg(port.to_string())
            .args(["40", "5", "5", "9"])
            .arg(SERVER_OUTPUT_FD.to_string());
        unsafe {
            cmd.pre_exec(move || {
                if dup2(writer.as_raw_fd(), SERVER_OUTPUT_FD) < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        self.server_process = Some(cmd.spawn().unwrap());
        self.server_output_pipe = Some(reader);
        self.committee_port_offsets = Some((1..=9).collect());

        loop {
            let mut probe = P::Client::new();
            if probe.request_state().is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    /// Stop the aggregation server process.
    pub fn kill_server(&mut self) {
        if let Some(mut child) = self.server_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Start `num_clients` client processes. Each expands a random seed to
    /// an input of length [`INPUT_LEN`], encrypts, and sends to the server.
    ///
    /// # Panics
    ///
    /// Panics if the client binary cannot be spawned.
    pub fn start_clients(&mut self, num_clients: usize) {
        let mut client = std::env::current_exe().unwrap();
        client.set_file_name("client");

        let mut rng = default_prg();
        for _ in 0..num_clients {
            let mut seed = [0u8; 32];
            populate_random_bytes(&mut seed, &mut rng);
            self.client_seeds.push(seed);
            // Spawn the client as a separate process.
            self.client_processes.push(
                Command::new(&client)
                    .arg(seed_to_hex(&seed))
                    .arg(INPUT_LEN.to_string())
                    .spawn()
                    .unwrap(),
            );
        }

        println!("{} clients started", num_clients);
    }

    /// Wait for all client processes to exit.
    pub fn wait_clients(&mut self) {
        for mut child in self.client_processes.drain(..) {
            let _ = child.wait();
        }
    }

    /// Start one committee process per port offset.
    ///
    /// # Panics
    ///
    /// Panics if [`Self::start_server`] has not run, or if the committee
    /// binary cannot be spawned.
    pub fn start_committee(&mut self) {
        assert!(
            self.committee_port_offsets.is_some(),
            "Committee port offsets are not set"
        );

        let mut committee = std::env::current_exe().unwrap();
        committee.set_file_name("committee");

        let port_offsets = self.committee_port_offsets.as_ref().unwrap();
        for port_offset in port_offsets {
            let port = STARTING_PORT + *port_offset;
            // Spawn the committee member as a separate process.
            self.committee_processes.push(
                Command::new(&committee)
                    .arg(port.to_string())
                    .spawn()
                    .unwrap(),
            );
        }

        println!("{} committee members started", port_offsets.len());
    }

    /// Wait for all committee processes to exit.
    pub fn wait_committee(&mut self) {
        for mut child in self.committee_processes.drain(..) {
            let _ = child.wait();
        }
    }

    /// Expand stored client seeds and store their elementwise sum as the
    /// expected output.
    pub fn collect_client_inputs(&mut self, expected_count: usize) -> Vec<Vec<u32>> {
        let inputs: Vec<Vec<u32>> = self
            .client_seeds
            .iter()
            .take(expected_count)
            .map(|seed| expand_seed(*seed, INPUT_LEN))
            .collect();

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

    /// Print whether the server output matches the expected sum.
    ///
    /// Call [`Self::collect_client_inputs`] first to populate the expected sum.
    pub fn output(&mut self) {
        let Some(ref mut pipe) = self.server_output_pipe else {
            println!("Server output: <no output available yet>");
            return;
        };
        let mut buf = Vec::new();
        pipe.read_to_end(&mut buf).unwrap();
        let output: Vec<u32> = crate::communicator::wire::decode(&buf).unwrap();

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

    /// End the run. Currently a no-op: party threads are not joined.
    pub fn teardown(&mut self) {}
}

/// Encode a 32-byte seed as 64 hex characters.
fn seed_to_hex(seed: &[u8; 32]) -> String {
    seed.iter().map(|b| format!("{b:02x}")).collect()
}

/// Expand a seed to `length` elements in `[0, 2^20)`.
fn expand_seed(seed: [u8; 32], length: usize) -> Vec<u32> {
    let mut rng = ChaCha20Rng::from_seed(seed);
    let mut input = vec![0u32; length];
    populate_random(&mut input, &mut rng);
    input.iter().map(|x| x % (1 << 20)).collect()
}
