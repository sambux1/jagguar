use crate::protocols::committee::Committee;
use crate::protocols::opa::server::OPAState;
use crate::communicator::Communicator;

pub struct OPACommittee {
    server_state: Option<OPAState>,
    communicator: Communicator,
}

impl Committee for OPACommittee {
    type ServerState = OPAState;

    fn new(port: u16) -> Self {
        // create a communicator on the passed port
        let communicator = Communicator::new(port);

        Self {
            server_state: None,
            communicator,
        }
    }

    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state.clone());
    }

    fn retrieve_inputs(&self) {
        // signal the server that we're ready to retrieve inputs
        let server_port = self.server_state.as_ref().unwrap().port;
        self.communicator.receive_from_server(server_port).expect("Failed to receive inputs from server");
    }

    fn aggregate(&self) {
        // placeholder
    }
}
