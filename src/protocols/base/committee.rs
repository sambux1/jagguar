/// Helper party: fetch per-member shares, aggregate, send the result back.
#[allow(async_fn_in_trait)]
pub trait Committee {
    /// Public state fetched from the server.
    type ServerState;

    /// `port` is this member's local bind port.
    fn new(port: u16) -> Self;
    /// Request public state over HTTP.
    async fn request_state(&mut self) -> std::io::Result<()>;
    fn set_server_state(&mut self, state: Self::ServerState);
    async fn retrieve_inputs(&mut self);
    fn aggregate(&mut self);
    async fn send_output(&mut self);
}
