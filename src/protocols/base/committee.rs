/// Helper party: fetch per-member shares, aggregate, send the result back.
pub trait Committee {
    /// Server public state (must match [`super::Server::State`]).
    type ServerState;

    /// `port` is this member's local bind port.
    fn new(port: u16) -> Self;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn retrieve_inputs(&mut self);
    fn aggregate(&mut self);
    fn send_output(&mut self);
}
