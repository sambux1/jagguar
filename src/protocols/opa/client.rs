use crate::protocols::client::Client;

pub struct OPAClient<Input> {
    input: Option<Input>,
}

impl<Input> OPAClient<Input> {
    pub fn new() -> Self {
        Self {
            input: None,
        }
    }

    pub fn get_input(&self) -> Option<&Input> {
        self.input.as_ref()
    }

    pub fn setup(&self) {
        // setup the client
    }
}

impl<Input> Client<Input> for OPAClient<Input> {
    fn set_input(&mut self, input: Input) {
        self.input = Some(input);
    }
}


