use crate::protocols::Protocol;

pub struct Simulator<P: Protocol> {
	_marker: core::marker::PhantomData<(P, P::Server, P::Client, P::Committee)>,
}

impl<P: Protocol> Simulator<P> {
	pub fn new() -> Self {
		Self { _marker: core::marker::PhantomData }
	}
}
