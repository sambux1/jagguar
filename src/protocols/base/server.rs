use crate::communicator::Communicator;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

/// Aggregation server: public parameters, input collection, and decode.
///
/// Communication is localhost TCP via [`Communicator`].
pub trait Server: Sized {
    type SetupParameters;
    /// Public state distributed to clients and committee members. Must be
    /// [`Clone`] so the simulator can hand out copies.
    type State: Clone;

    /// Construct a new server.
    fn new(args: Self::SetupParameters) -> Self;
    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    /// Final aggregation from a state snapshot.
    ///
    /// Takes `&State` so communicator callbacks can run this without a
    /// `&mut Self`.
    fn aggregate(state: &Self::State);
    /// Local-port offsets of committee members, relative to the simulator
    /// base port.
    fn get_committee_port_offsets(&self) -> Vec<u16>;

    /// Install a channel for the decoded output (simulator hook).
    ///
    /// Default is a no-op. The payload type is `Vec<u32>` for the current
    /// simulator.
    fn set_output_channel(&mut self, _sender: Sender<Vec<u32>>) {}

    fn set_communicator(&mut self, comm: Communicator);
    fn get_communicator(&mut self) -> &mut Communicator;

    /// Called after the communicator is bound, before the listen loop.
    fn on_communicator_setup(&mut self, _port: u16) {
        // default: do nothing
    }

    /// Bind `port`, publish [`Self::State`], and run the listen loop until
    /// `shutdown` is set.
    ///
    /// # Panics
    ///
    /// Panics if the listener fails to bind or the listen loop returns an
    /// error.
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
        let _ = state_sender
            .send(self.get_state().clone())
            .expect("failed to send state");

        self.get_communicator()
            .listen_loop()
            .expect("failed to listen");
    }
}
