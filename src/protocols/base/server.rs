pub trait Server {
    type SetupParameters;

    fn setup(&mut self, args: Self::SetupParameters);
    fn aggregate(&self);
}
