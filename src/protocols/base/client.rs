pub trait Client<T> {
    fn set_input(&mut self, input: Vec<T>);
    fn encrypt_input(&self);
}
