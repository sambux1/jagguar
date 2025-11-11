use crate::protocols::client::Client;
use crate::protocols::opa::server::OPAState;
use crate::crypto::SeedHomomorphicPRG;

pub struct OPAClient<T> {
    input: Option<Vec<T>>,
    server_state: Option<OPAState>,
}

impl<T> OPAClient<T> {
    pub fn new() -> Self {
        Self {
            input: None,
            server_state: None,
        }
    }

    pub fn get_input(&self) -> Option<&Vec<T>> {
        self.input.as_ref()
    }

    pub fn set_server_state(&mut self, state: OPAState) {
        self.server_state = Some(state);
    }

    pub fn setup(&self) {
        // setup the client
    }

    fn encode_input(&self) {
        // do nothing yet
    }
}

impl<T> Client<T> for OPAClient<T> {
    fn set_input(&mut self, input: Vec<T>) {
        self.input = Some(input);
    }

    fn encrypt_input(&self) {
        // assert that the input and server state are set
        assert!(self.input.is_some(), "OPA client input is not set.");
        assert!(self.server_state.is_some(), "OPA client server state is not set.");

        // instantiate the SHPRG by expanding the seed into the public parameter
        let shprg = SeedHomomorphicPRG::new_from_public_seed(
            self.server_state.as_ref().unwrap().succinct_seed);
        
        // encode the input
        self.encode_input();

        // generate the mask from the SHPRG
    }
}


