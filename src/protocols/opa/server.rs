use crate::protocols::server::Server;
use crate::crypto::SeedHomomorphicPRG;
use crate::crypto::prg::populate_random_bytes;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[derive(Copy, Clone)]
pub struct OPASetupParameters {
    security_parameter: u64,
    corruption_threshold: u64,
    reconstruction_threshold: u64,
    committee_size: u64,
}

impl OPASetupParameters {
    pub fn new(
        security_parameter: u64,
        corruption_threshold: u64,
        reconstruction_threshold: u64,
        committee_size: u64
    ) -> Self {
        Self {
            security_parameter,
            corruption_threshold,
            reconstruction_threshold,
            committee_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OPAState {
    pub succinct_seed: [u8; 32],
    pub security_parameter: u64,
    pub corruption_threshold: u64,
    pub reconstruction_threshold: u64,
    pub committee_size: u64,
}

pub struct OPAServer {
    setup_parameters: OPASetupParameters,
    state: OPAState,
    public_parameter: Vec<Vec<crate::crypto::F128>>,
}

impl OPAServer {
    pub fn new(setup_parameters: OPASetupParameters) -> Self {
        // default initialization, delegate real work to setup function
        let mut server = Self {
            setup_parameters,
            state: OPAState {
                succinct_seed: [0u8; 32],
                security_parameter: 0,
                corruption_threshold: 0,
                reconstruction_threshold: 0,
                committee_size: 0,
            },
            public_parameter: Vec::new(),
        };
        server.setup(server.setup_parameters);
        server
    }
}

impl Server for OPAServer {
    type SetupParameters = OPASetupParameters;
    type State = OPAState;

    fn setup(&mut self, args: Self::SetupParameters) {
        self.setup_parameters = args;

        // sample the public parameter seed
        let mut rng = ChaCha20Rng::from_entropy();
        let mut succinct_seed = [0u8; 32];
        populate_random_bytes(&mut succinct_seed, &mut rng);

        // set the public state
        self.state = OPAState {
            succinct_seed,
            security_parameter: self.setup_parameters.security_parameter,
            corruption_threshold: self.setup_parameters.corruption_threshold,
            reconstruction_threshold: self.setup_parameters.reconstruction_threshold,
            committee_size: self.setup_parameters.committee_size,
        };

        // sample the public parameter from the succinct seed
        let shprg = SeedHomomorphicPRG::new_from_public_seed(succinct_seed);
        let public_parameter = shprg.get_public_parameter();
        self.public_parameter = public_parameter.clone();
    }

    fn get_state(&self) -> &Self::State {
        &self.state
    }

    fn aggregate(&self) {
        // aggregate the inputs
    }
}