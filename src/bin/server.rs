//! Aggregation server.
//!
//! Constructs a server from setup parameters, starts it, and stays running
//! so clients and committee members can connect.

use jagguar::protocols::Server;
use jagguar::protocols::opa::{OPAServer, OPASetupParameters};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let output_fd = if args.len() == 6 {
        Some(args.pop().unwrap().parse().expect("invalid output fd"))
    } else {
        None
    };
    let args: [String; 5] = args.try_into().expect(
        "usage: server <port> <security_parameter> <corruption_threshold> \
         <reconstruction_threshold> <committee_size> [output_fd]",
    );
    let mut server = OPAServer::new(OPASetupParameters::new(
        args[1].parse().unwrap(),
        args[2].parse().unwrap(),
        args[3].parse().unwrap(),
        args[4].parse().unwrap(),
    ));
    if let Some(fd) = output_fd {
        server.set_output_fd(fd);
    }
    server.setup_communicator(args[0].parse().unwrap());

    // keep the process alive, the server serves until it is stopped
    std::thread::park();
}
