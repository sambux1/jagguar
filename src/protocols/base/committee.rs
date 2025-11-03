pub trait Committee {
    fn retrieve_inputs(&self);
    fn aggregate(&self);
}
