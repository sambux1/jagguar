//! Aggregation committee member.
//!
//! Fetches public state and this member's shares, aggregates them, and
//! sends the result to the server.

use jagguar::protocols::Committee;
use jagguar::protocols::opa::OPACommittee;

/// Parse the port, fetch shares, aggregate, and send the result to the server.
fn main() {
    let port = std::env::args()
        .nth(1)
        .expect("usage: committee <port>")
        .parse()
        .expect("invalid port");

    pollster::block_on(async {
        let mut committee: OPACommittee = OPACommittee::new(port);
        if committee.request_state().await.is_err() {
            eprintln!("Error: cannot connect to server");
            std::process::exit(1);
        }
        committee.retrieve_inputs().await;
        committee.aggregate();
        committee.send_output().await;
    });
}
