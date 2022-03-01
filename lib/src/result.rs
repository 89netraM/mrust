use super::{Applicative, Functor, Monad};

impl<A, E> Functor<A> for Result<A, E> {
	type Map<B> = Result<B, E>;

	fn map<B, F: FnOnce(A) -> B>(self, f: F) -> Self::Map<B> {
		self.map(f)
	}
}

impl<A, E> Applicative<A> for Result<A, E> {
	type Apply<B> = Result<B, E>;

	fn pure(a: A) -> Self {
		Ok(a)
	}

	fn ap<B, F: FnOnce(A) -> B>(self, f: Self::Apply<F>) -> Self::Apply<B> {
		self.and_then(|a| f.map(|f| f(a)))
	}
}

impl<A, E> Monad<A> for Result<A, E> {
	type Bind<B> = Result<B, E>;

	fn bind<B, F: FnOnce(A) -> Self::Bind<B>>(self, f: F) -> Self::Bind<B> {
		self.and_then(f)
	}
}
