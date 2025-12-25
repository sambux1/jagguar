use aggregation::protocols::opa::{OPA, OPASetupParameters};
use aggregation::simulator::Simulator;

fn main() {
    // create the simulator
	let mut sim: Simulator<OPA<u32>> = Simulator::new();

    // create the (single) server
    // TODO: don't depend on OPA parameters explicitly
    let server_parameters = OPASetupParameters::new(40, 16, 16, 31);
    sim.start_server(server_parameters);

    // create many clients
    sim.start_clients(10);

    println!("Running the simulator...");

    // let the simulator run for 5 seconds
    std::thread::sleep(std::time::Duration::from_secs(5));

    // teardown the simulator
    sim.teardown();
}
