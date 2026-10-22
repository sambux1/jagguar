//! Example localhost OPA run (`cargo simulator`).

use jagguar::protocols::opa::{OPA, OPASetupParameters};
use jagguar::simulator::Simulator;

fn main() {
    // create the simulator
    let mut sim: Simulator<OPA<u32>> = Simulator::new();

    // create the (single) server
    // TODO: don't depend on OPA parameters explicitly
    let server_parameters = OPASetupParameters::new(40, 5, 5, 9);
    sim.start_server(server_parameters);

    // create many clients
    let num_clients = 1000;
    sim.start_clients(num_clients);
    println!("Waiting for client uploads...");
    sim.wait_clients();
    let client_inputs = sim.collect_client_inputs(num_clients);
    println!("Collected {} client inputs", client_inputs.len());

    // connect the committee members, disjoint from clients
    sim.start_committee();
    sim.wait_committee();

    sim.output();
    sim.kill_server();
}
