use crate::communicator::Communicator;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub trait Server {
    type SetupParameters;
    type State;

    fn new(args: Self::SetupParameters) -> Self;
    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    fn aggregate(&self);

    fn set_communicator(&mut self, comm: Communicator);
    fn get_communicator(&mut self) -> &mut Communicator;

    fn setup_communicator(&mut self, port: u16, shutdown: Arc<AtomicBool>) {
        let mut comm = Communicator::new(port);
        comm.set_shutdown_flag(shutdown);
        comm.start_server().expect("failed to start server");
        self.set_communicator(comm);
        self.get_communicator().listen_loop().expect("failed to listen");
        println!("Server listening on port {}", port);
    }
}
