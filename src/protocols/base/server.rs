use crate::communicator::Communicator;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::mpsc::Sender;

pub trait Server: Sized {
    type SetupParameters;
    type State: Clone;

    fn new(args: Self::SetupParameters) -> Self;
    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    fn aggregate(&self);

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
