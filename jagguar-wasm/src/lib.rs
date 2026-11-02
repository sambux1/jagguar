use jagguar::communicator::{DefaultTransport, Transport};
use jagguar::protocols::opa::{OPAClient, OPACommittee, OPAState};
use jagguar::protocols::{Client, Committee};
use wasm_bindgen::prelude::*;

fn js_error(error: std::io::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn format_state(state: &OPAState) -> String {
    let seed = state
        .succinct_seed
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "port: {}\n\
         security_parameter: {}\n\
         corruption_threshold: {}\n\
         reconstruction_threshold: {}\n\
         committee_size: {}\n\
         committee_port_offsets: {:?}\n\
         succinct_seed: {}",
        state.port,
        state.security_parameter,
        state.corruption_threshold,
        state.reconstruction_threshold,
        state.committee_size,
        state.committee_port_offsets,
        seed
    )
}

/// Fetch `GET /state` through the wasm transport and return a printable summary.
#[wasm_bindgen]
pub async fn get_state() -> Result<String, JsValue> {
    let bytes = DefaultTransport::default()
        .request_state()
        .await
        .map_err(js_error)?;
    let state = OPAState::from_reply(&bytes).map_err(js_error)?;
    Ok(format_state(&state))
}

#[wasm_bindgen]
pub async fn run_client(input: Vec<u32>) -> Result<(), JsValue> {
    let mut client = OPAClient::<u32>::new();
    client.request_state().await.map_err(js_error)?;
    client.set_input(input);
    client.encrypt_input();
    client.send_input().await;
    Ok(())
}

#[wasm_bindgen]
pub async fn run_committee(port: u16) -> Result<(), JsValue> {
    let mut committee: OPACommittee = OPACommittee::new(port);
    committee.request_state().await.map_err(js_error)?;
    committee.retrieve_inputs().await;
    committee.aggregate();
    committee.send_output().await;
    Ok(())
}
