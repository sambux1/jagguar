pub trait Committee {
    type ServerState;

    fn new(port: u16) -> Self;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn retrieve_inputs(&mut self);
    fn aggregate(&mut self);
    fn send_output(&mut self);
}
