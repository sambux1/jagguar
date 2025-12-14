use aggregation::protocols::opa::OPA;
use aggregation::simulator::Simulator;

fn main() {
	let _sim: Simulator<OPA<u32>> = Simulator::new();

	println!("Initialized simulator.");
}
