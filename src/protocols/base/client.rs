/// Input party: receive public state, encrypt, send to the server.
#[allow(async_fn_in_trait)]
pub trait Client<T> {
    /// Encrypted payload produced by [`Self::encrypt_input`].
    type Output;
    /// Public state fetched from the server.
    type ServerState;

    fn new() -> Self;
    /// Request public state over HTTP.
    async fn request_state(&mut self) -> std::io::Result<()>;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&mut self);
    /// Send the encrypted input to the server.
    async fn send_input(&mut self);
}
