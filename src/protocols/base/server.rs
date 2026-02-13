use crate::communicator::Communicator;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub trait Server: Sized {
    type SetupParameters;
    type State: Clone;

    /// Construct a new server.
    fn new(args: Self::SetupParameters) -> Self;
    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    /// Aggregate using the provided state. This allows callbacks to call aggregation
    /// without needing access to the server instance.
    fn aggregate(state: &Self::State);
    fn get_committee_port_offsets(&self) -> Vec<u16>;

    /// Optionally allow the caller (e.g., simulator) to install an output channel
    /// that the server can use to send results back to the main thread.
    /// Default implementation is a no-op so servers that don't use it can ignore it.
    fn set_output_channel(&mut self, _sender: Sender<Vec<u32>>) {}

    fn set_communicator(&mut self, comm: Communicator);
    fn get_communicator(&mut self) -> &mut Communicator;

    fn on_communicator_setup(&mut self, _port: u16) {
        // default: do nothing
    }

    fn setup_communicator(
        &mut self,
        port: u16,
        shutdown: Arc<AtomicBool>,
        state_sender: Sender<Self::State>,
    ) {
        let mut comm = Communicator::new(port);
        comm.set_shutdown_flag(shutdown);
        comm.start_server().expect("failed to start server");
        self.set_communicator(comm);
        
        // call the derived class's on_communicator_setup method
        self.on_communicator_setup(port);
        println!("Server listening on port {}", port);
        
        // send the state back to signal that the server is ready
        let _ = state_sender.send(self.get_state().clone()).expect("failed to send state");
        
        self.get_communicator().listen_loop().expect("failed to listen");
    }
}
