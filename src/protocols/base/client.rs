pub trait Client<T> {
    type Output;
    type ServerState;
    
    fn new() -> Self;
    fn set_server_state(&mut self, state: Self::ServerState);
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&mut self);
    fn send_input(&mut self, port: u16);
}
