/// Input party: receive public state, encrypt, send to the server.
pub trait Client<T> {
    /// Encrypted payload produced by [`Self::encrypt_input`].
    type Output;
    /// Public state fetched from the server.
    type ServerState;

    fn new() -> Self;
    /// Request public state over HTTP.
    fn request_state(&mut self) -> std::io::Result<()>;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&mut self);
    /// Send the encrypted input to the server.
    fn send_input(&mut self);
}
