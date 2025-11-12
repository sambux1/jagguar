use crate::protocols::client::Client;
use crate::protocols::opa::server::OPAState;
use crate::crypto::SeedHomomorphicPRG;
use crate::util::packing::pack_vector;

pub struct OPAClient<T> {
    input: Option<Vec<T>>,
    server_state: Option<OPAState>,
}

impl<T: Copy + Into<u64> + num_traits::FromPrimitive> OPAClient<T> {
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

    fn encode_input(&self) -> Vec<u64>
    where
        T: Copy + Into<u64> + num_traits::FromPrimitive,
    {
        let input = self.input.as_ref().expect("OPA client input must be set.");
        pack_vector(&input)
    }
}

impl<T: Copy + Into<u64> + num_traits::FromPrimitive> Client<T> for OPAClient<T> {
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
        let _encoded_input = self.encode_input();

        // generate the mask from the SHPRG
    }
}


