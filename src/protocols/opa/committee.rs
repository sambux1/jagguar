use crate::protocols::committee::Committee;

pub struct OPACommittee;

impl OPACommittee {
    pub fn new() -> Self {
        Self
    }
}

impl Committee for OPACommittee {
    fn retrieve_inputs(&self) {
        // placeholder
    }

    fn aggregate(&self) {
        // placeholder
    }
}


