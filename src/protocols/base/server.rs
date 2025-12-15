pub trait Server {
    type SetupParameters;
    type State;

    fn new(args: Self::SetupParameters) -> Self;
    fn setup(&mut self, args: Self::SetupParameters);
    fn get_state(&self) -> &Self::State;
    fn aggregate(&self);
}
