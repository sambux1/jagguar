pub trait Client<T> {
    type Output;
    
    fn new() -> Self;
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&mut self) -> Self::Output;
}
