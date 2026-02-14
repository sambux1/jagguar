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
    let num_clients = 25;
    sim.start_clients(num_clients);

    // collect all client inputs from the channel
    // loop over all received messages and add them to a list
    let client_inputs = sim.collect_client_inputs(num_clients);
    println!("Collected {} client inputs", client_inputs.len());

    println!("Running the simulator...");

    // let the simulator run for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(5));

    // connect the committee members, disjoint from clients
    sim.start_committee();

    std::thread::sleep(std::time::Duration::from_secs(1));

    sim.output();

    // teardown the simulator
    sim.teardown();
}
