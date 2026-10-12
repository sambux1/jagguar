use std::sync::{Arc, Mutex};

/// Party endpoint for one simulated aggregation.
///
/// `port` is this party's identity. Payloads move over HTTP.
pub struct Communicator {
    port: u16,
    received_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_messages: Arc<Mutex<Vec<Vec<u8>>>>,
    committee_fetch_callback: Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>>,
    committee_expected_size: Option<usize>,
    committee_complete_callback: Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
}

impl Communicator {
    /// Communicator forparty with identity `port`.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            committee_messages: Arc::new(Mutex::new(Vec::new())),
            committee_fetch_callback: None,
            committee_expected_size: None,
            committee_complete_callback: None,
        }
    }

    /// Party identity.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Builds the `POST /committee-fetch` response from the raw request body.
    pub fn set_committee_fetch_callback<F>(&mut self, callback: F)
    where
        F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        self.committee_fetch_callback = Some(Arc::new(callback));
    }

    /// How many `committee`-prefixed messages trigger the complete callback.
    pub fn set_committee_expected_size(&mut self, size: usize) {
        self.committee_expected_size = Some(size);
    }

    /// Called with a snapshot of committee messages once the expected count
    /// is reached. The queue is cleared first.
    pub fn set_committee_complete_callback<F>(&mut self, callback: F)
    where
        F: Fn(Vec<Vec<u8>>) + Send + Sync + 'static,
    {
        self.committee_complete_callback = Some(Arc::new(callback));
    }

    /// Queued client payloads (shared with callbacks).
    pub fn get_received_messages(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.received_messages)
    }

    /// Queued `committee`-prefixed payloads (shared with callbacks).
    pub fn get_committee_messages(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.committee_messages)
    }

    /// Committee queue, expected count, and complete callback.
    pub(crate) fn get_committee_info(
        &self,
    ) -> (
        Arc<Mutex<Vec<Vec<u8>>>>,
        Option<usize>,
        Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
    ) {
        (
            Arc::clone(&self.committee_messages),
            self.committee_expected_size,
            self.committee_complete_callback.clone(),
        )
    }

    /// Callback registered with [`Self::set_committee_fetch_callback`].
    pub(crate) fn get_committee_fetch_callback(
        &self,
    ) -> Option<Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>> {
        self.committee_fetch_callback.clone()
    }

    /// Queue one committee payload. When the queue reaches `expected`, take a
    /// snapshot, clear it, and run the complete callback.
    pub(crate) fn push_committee_message(
        committee_messages: &Arc<Mutex<Vec<Vec<u8>>>>,
        expected: Option<usize>,
        callback: &Option<Arc<dyn Fn(Vec<Vec<u8>>) + Send + Sync>>,
        message: Vec<u8>,
    ) {
        let mut committee_queue = committee_messages.lock().unwrap();
        committee_queue.push(message);
        let current_len = committee_queue.len();
        println!(
            "Committee message queued (total: {} committee messages)",
            current_len
        );

        if let Some(expected) = expected {
            if current_len == expected {
                let batch = committee_queue.clone();
                committee_queue.clear();
                drop(committee_queue);
                if let Some(cb) = callback {
                    cb(batch);
                }
            }
        }
    }
}
