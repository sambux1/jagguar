use std::io::Write;

use crate::protocols::client::Client;
use crate::protocols::opa::server::OPAState;
use crate::crypto::{F256, FieldBytes, OUTER_MODULUS_BITS, SeedHomomorphicPRG, Shamir, field_to_bytes};
use crate::crypto::prg::{populate_random, default_prg};
use crate::util::packing::pack_vector;
use crate::communicator::Communicator;

pub const NUM_PARTIES_UPPER_BOUND: u64 = 1 << 20;
/// Exclusive upper bound on input values; must stay below the SHPRG outer modulus 2^128.
pub const MAX_FIELD_VALUE: u128 = 1u128 << (OUTER_MODULUS_BITS - 1);
/// Fixed ciphertext length matching the SHPRG output size.
pub const OUTPUT_LEN: usize = 4096;

pub struct OPAClient<T> {
    input: Option<Vec<T>>,
    server_state: Option<OPAState>,
    encrypted_output: Option<(Vec<u128>, Vec<Vec<FieldBytes>>)>,
    #[cfg(test)]
    last_seed: Option<Vec<u128>>,
}

impl<T: Copy + Into<u32> + num_traits::FromPrimitive> OPAClient<T> {
    pub fn get_input(&self) -> Option<&Vec<T>> {
        self.input.as_ref()
    }

    /// Helper for tests and debugging: decode an encoded (already unmasked)
    /// aggregate using the client's view of the server state.
    pub fn decode_output(&self, output: Vec<u128>) -> Vec<u32> {
        self.server_state
            .as_ref()
            .expect("OPA client server state must be set.")
            .decode_output(output)
    }

    pub fn setup(&self) {
        // setup the client
    }

    fn encode_input(&self) -> Vec<u128>
    where
        T: Copy + Into<u32> + num_traits::FromPrimitive,
    {
        let input = self.input.as_ref().expect("OPA client input must be set.");
        let packed = pack_vector(&input);
        
        // compute 2^kappa and (2^kappa * n)
        let kappa: u32 = self.server_state.as_ref().unwrap().security_parameter as u32;
        let two_to_kappa: u128 = 1u128 << kappa;
        let two_to_kappa_times_n: u128 = two_to_kappa * (NUM_PARTIES_UPPER_BOUND as u128);
        let mask: u64 = if kappa >= 64 { u64::MAX } else { (two_to_kappa - 1) as u64 };
        
        // compute a vector of random numbers in [0, 2^kappa)
        let mut random_numbers = vec![0u64; packed.len()];
        let mut rng = default_prg();
        populate_random(&mut random_numbers, &mut rng);
        // mask away the higher order bits of the random numbers
        random_numbers = random_numbers.iter()
            .map(|&x| x & mask).collect();
        
        // encoded = (2^kappa * n * x) + r + 2^kappa
        packed
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let r: u128 = random_numbers[i] as u128;
                two_to_kappa_times_n * (x as u128) + r + two_to_kappa
            })
            .collect()
    }
}

impl<T: Copy + Into<u32> + num_traits::FromPrimitive> Client<T> for OPAClient<T> {
    type Output = (Vec<u128>, Vec<Vec<FieldBytes>>);
    type ServerState = OPAState;

    fn new() -> Self {
        Self {
            input: None,
            server_state: None,
            encrypted_output: None,
            #[cfg(test)]
            last_seed: None,
        }
    }
    
    fn set_input(&mut self, input: Vec<T>) {
        for (i, value) in input.iter().enumerate() {
            assert!(
                ((*value).into() as u128) < MAX_FIELD_VALUE,
                "input[{i}] exceeds maximum field value 2^127"
            );
        }
        self.input = Some(input);
    }

    fn set_server_state(&mut self, state: Self::ServerState) {
        self.server_state = Some(state);
    }

    fn encrypt_input(&mut self) {
        // assert that the input and server state are set
        assert!(self.input.is_some(), "OPA client input is not set.");
        assert!(self.server_state.is_some(), "OPA client server state is not set.");

        // instantiate the SHPRG by expanding the seed into the public parameter
        let shprg = SeedHomomorphicPRG::new_from_public_seed(
            self.server_state.as_ref().unwrap().succinct_seed);
        
        let mut encoded_input = self.encode_input();
        let payload_len = encoded_input.len();
        assert!(payload_len <= OUTPUT_LEN, "encoded input exceeds SHPRG output length");
        encoded_input.resize(OUTPUT_LEN, 0);

        let mask = shprg.expand();

        let masked_input: Vec<u128> = encoded_input
            .iter()
            .zip(mask.iter())
            .map(|(&x, &m)| x.wrapping_add(m))
            .collect();
        
        // secret share the SHPRG seed using Shamir secret sharing over F256
        let state = self.server_state.as_ref().unwrap();
        let num_shares = state.committee_size as usize;
        let threshold = state.reconstruction_threshold as usize;
        let shamir = Shamir::<F256>::new(num_shares, threshold);
        let seed = shprg.get_seed();

        // organize as shares[party_index][seed_index] = y (stored as 32-byte field element)
        let mut shares: Vec<Vec<FieldBytes>> = vec![Vec::with_capacity(seed.len()); num_shares];
        for &secret in seed.iter() {
            let secret_shares = shamir.share(F256::from(secret), &mut default_prg())
                .expect("Shamir share failed");
            for i in 0..num_shares {
                let (_x, y) = secret_shares[i];
                shares[i].push(field_to_bytes(y));
            }
        }

        // store the seed for tests only
        #[cfg(test)]
        {
            self.last_seed = Some(seed.clone());
        }

        // save to state
        self.encrypted_output = Some((masked_input.clone(), shares.clone()));
    }

    // send the encrypted input to the server
    fn send_input(&mut self, port: u16) {
        // create a communicator on the passed port
        let communicator = Communicator::new(port);

        // establish a connection to the server through the communicator
        let server_port = self.server_state.as_ref().unwrap().port;
        let (masked_input, shares) = self.encrypted_output.as_ref()
            .expect("Must call encrypt_input before send_input");

        // serialize: [payload_len u32][masked_input...][num_shares u32][share...]
        let mut data = Vec::new();
        let payload_len = pack_vector(self.input.as_ref().unwrap()).len() as u32;
        data.write_all(&payload_len.to_le_bytes()).unwrap();
        for &x in masked_input {
            data.write_all(&x.to_le_bytes()).unwrap();
        }
        // record the number of shares (committee members)
        data.write_all(&(shares.len() as u32).to_le_bytes()).unwrap();
        for share in shares {
            // each share: length + 32-byte field elements
            data.write_all(&(share.len() as u32).to_le_bytes()).unwrap();
            for x in share {
                data.write_all(x).unwrap();
            }
        }

        if let Err(e) = communicator.send_to_server(server_port, &data) {
            eprintln!("Failed to send to server: {}", e);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::server::Server;
    use crate::protocols::opa::{OPAServer, OPASetupParameters};
    use crate::crypto::{F256, Shamir, field_from_bytes, field_low_u128};

    #[test]
    // test that decode(encode(x)) = x
    fn test_encoding() {
        let input : Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8];
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

    #[test]
    // test that the encryption produces the correct secret shares
    fn test_encryption() {
        let input : Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8];
    
        let opa_server = OPAServer::new(OPASetupParameters::new(40, 16, 16, 31));
        let state = opa_server.get_state();
        
        let mut opa_client = OPAClient::<u32>::new();
        opa_client.set_input(input);
        opa_client.set_server_state(state.clone());

        // encrypt the input
        opa_client.encrypt_input();
        let (_masked_input, shares) = opa_client.encrypted_output.as_ref().unwrap();
        
        // reconstruct the seed from the secret shares
        let shamir = Shamir::<F256>::new(
            state.committee_size as usize,
            state.reconstruction_threshold as usize
        );
        // shares is organized as shares[party_index][seed_index] = y.
        // reconstruct each seed component j from all parties' shares at index j.
        let num_parties = shares.len();
        let seed_len = shares[0].len();
        let mut reconstructed_seed: Vec<u128> = Vec::with_capacity(seed_len);
        for j in 0..seed_len {
            let mut pairs: Vec<(F256, F256)> = Vec::with_capacity(num_parties);
            for i in 0..num_parties {
                let x = F256::from((i as u64) + 1);
                let y = field_from_bytes(&shares[i][j]);
                pairs.push((x, y));
            }
            let s = shamir.reconstruct(&pairs).unwrap();
            reconstructed_seed.push(field_low_u128(s));
        }

        // get the last seed from the client, generated during encryption
        let last_seed = opa_client.last_seed.unwrap();
        
        assert_eq!(last_seed, reconstructed_seed);
    }
}