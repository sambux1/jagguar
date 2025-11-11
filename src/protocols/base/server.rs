pub trait Server {
    type SetupParameters;
    type State;

    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    fn aggregate(&self);
}
