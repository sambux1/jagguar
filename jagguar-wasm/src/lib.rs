use jagguar::protocols::opa::{OPAClient, OPACommittee};
use jagguar::protocols::{Client, Committee};
use wasm_bindgen::prelude::*;

fn js_error(error: std::io::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
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
