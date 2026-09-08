/// Input party: receive public state, encrypt, send to the server.
pub trait Client<T> {
    /// Encrypted payload produced by [`Self::encrypt_input`].
    type Output;
    /// Server public state (must match [`super::Server::State`]).
    type ServerState;

    fn new() -> Self;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&mut self);
    /// Send the encrypted input. `port` is this client's local bind port.
    fn send_input(&mut self, port: u16);
}
