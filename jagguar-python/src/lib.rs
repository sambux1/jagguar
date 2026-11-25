//! Python bindings for a native Jagguar aggregation client.
//!
//! Same client flow as `jagguar-wasm`, exposed as a [`Client`] class plus
//! `run_client` and `get_state` convenience functions.

use jagguar_core::communicator::{DefaultTransport, Transport};
use jagguar_core::protocols::Client as _;
use jagguar_core::protocols::opa::{OPAClient, OPAState};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

fn py_io_error(error: std::io::Error) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
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

/// OPA input party. Talks to a server already listening on `127.0.0.1:8080`.
#[pyclass]
struct Client {
    inner: OPAClient<u32>,
}

#[pymethods]
impl Client {
    #[new]
    fn new() -> Self {
        Self {
            inner: OPAClient::<u32>::new(),
        }
    }

    /// Fetch public state from the server.
    fn request_state(&mut self) -> PyResult<()> {
        pollster::block_on(self.inner.request_state()).map_err(py_io_error)
    }

    /// Store the plaintext input vector. Values must fit in `u32`.
    fn set_input(&mut self, input: Vec<u32>) {
        self.inner.set_input(input);
    }

    /// Encode, mask, and secret-share the stored input.
    fn encrypt_input(&mut self) {
        self.inner.encrypt_input();
    }

    /// Send the encrypted input to the server.
    fn send_input(&mut self) {
        pollster::block_on(self.inner.send_input());
    }

    /// Fetch state, encrypt `input`, and send it. Same as [`run_client`].
    fn run(&mut self, input: Vec<u32>) -> PyResult<()> {
        self.request_state()?;
        self.set_input(input);
        self.encrypt_input();
        self.send_input();
        Ok(())
    }
}

/// Fetch `GET /state` and return a printable summary.
#[pyfunction]
fn get_state() -> PyResult<String> {
    let bytes =
        pollster::block_on(DefaultTransport::default().request_state()).map_err(py_io_error)?;
    let state = OPAState::from_reply(&bytes).map_err(py_io_error)?;
    Ok(format_state(&state))
}

/// Construct a client, fetch state, encrypt `input`, and send it.
#[pyfunction]
fn run_client(input: Vec<u32>) -> PyResult<()> {
    Client::new().run(input)
}

#[pymodule]
fn jagguar(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(get_state, m)?)?;
    m.add_function(wrap_pyfunction!(run_client, m)?)?;
    Ok(())
}
