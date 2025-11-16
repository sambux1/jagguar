use crate::protocols::client::Client;
use crate::protocols::opa::server::OPAState;
use crate::crypto::{F128, SeedHomomorphicPRG};
use crate::crypto::prg::{populate_random, default_prg};
use crate::util::packing::{pack_vector, unpack_vector};
use ark_ff::PrimeField;

// TODO: temporary upper bound on number of parties
pub const NUM_PARTIES_UPPER_BOUND: u64 = 1024;

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

    fn encode_input(&self) -> Vec<F128>
    where
        T: Copy + Into<u64> + num_traits::FromPrimitive,
    {
        let input = self.input.as_ref().expect("OPA client input must be set.");
        let packed = pack_vector(&input);
        
        // compute 2^kappa and (2^kappa * n)
        let two_to_kappa = 1u64 << (self.server_state.as_ref().unwrap().security_parameter);
        let two_to_kappa_times_n = two_to_kappa * NUM_PARTIES_UPPER_BOUND;
        let two_to_kappa_f = F128::from(two_to_kappa);
        let two_to_kappa_times_n_f = F128::from(two_to_kappa_times_n);
        
        // compute a vector of random numbers in [0, 2^kappa)
        let mut random_numbers = vec![0u64; packed.len()];
        let mut rng = default_prg();
        populate_random(&mut random_numbers, &mut rng);
        // mask away the higher order bits of the random numbers
        random_numbers = random_numbers.iter()
            .map(|x| x & (two_to_kappa - 1)).collect();
        
        // encoded = (2^kappa * n * x) + r + 2^kappa
        let encoded = packed.iter().zip(random_numbers.iter())
            .map(|(x, y)| two_to_kappa_times_n_f * F128::from(*x) + F128::from(*y) + two_to_kappa_f)
            .collect();
        
        // return the encoded input
        encoded
    }

    fn decode_output(&self, output: Vec<F128>) -> Vec<T>
    where
        T: num_traits::FromPrimitive,
    {
        // first, convert the encoded output to u128
        let output_u128: Vec<u128> = output.iter()
            .map(|x| {
                let bi = (*x).into_bigint(); // consume copy, get BigInt with 2 limbs
                let limbs = bi.0;
                (limbs[0] as u128) | ((limbs[1] as u128) << 64)
            })
            .collect();
 
        // compute 2^kappa and (2^kappa * n)
        let two_to_kappa = 1u64 << (self.server_state.as_ref().unwrap().security_parameter);
        let two_to_kappa_times_n = two_to_kappa * NUM_PARTIES_UPPER_BOUND;

        // decoded = ceil(encoded / (2^kappa * n)) - 1
        let denom = two_to_kappa_times_n as u128;
        let decoded: Vec<u64> = output_u128.iter()
            .map(|x| {
                let q = *x / denom;
                let r = *x % denom;
                let ceil = q + if r != 0 { 1 } else { 0 };
                (ceil as u64) - 1
            })
            .collect();
 
        // unpack the elements of the decoded vector into the target type
        let result: Vec<T> = unpack_vector(&decoded);
        
        // return the decoded output
        result
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::server::Server;
    use crate::protocols::opa::{OPAServer, OPASetupParameters};

    #[test]
    // test that decode(encode(x)) = x
    fn test_encoding() {
        let input = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let expected = input.clone();

        let opa_server = OPAServer::new(OPASetupParameters::new(40, 16, 16, 31));
        let state = opa_server.get_state();

        let mut opa_client = OPAClient::<u32>::new();
        opa_client.set_input(input);
        opa_client.set_server_state(state.clone());

        let encoded_input = opa_client.encode_input();
        let decoded_input = opa_client.decode_output(encoded_input);
        assert_eq!(expected, decoded_input);
    }
}