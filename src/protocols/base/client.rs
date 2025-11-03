pub trait Client<Input> {
    fn set_input(&mut self, input: Input);
}
