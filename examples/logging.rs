use mrust::*;

fn main() {
	let logged = log_test(true);
	println!("Logs:");
	let result = logged.run();
	println!("Result: {result}");
}

/// Logs the path of the function, and returns a value.
#[monadic]
fn log_test(a: bool) -> Logging<i32> {
	Logging::log("Enter function")?;
	if a {
		Logging::log("Is statement A")?;
	}?;
	if !a {
		Logging::log("If statement B")?;
	}?;
	ret(2)
}

/// Simple logging monad.
struct Logging<A> {
	value: A,
	log: Vec<String>,
}

impl Logging<()> {
	/// Logs a singled message `msg`.
	fn log<M: ToString>(msg: M) -> Self {
		Self {
			value: (),
			log: vec![msg.to_string()],
		}
	}
}

impl<A> Logging<A> {
	/// Prints all logs and returns the computed value.
	fn run(&self) -> &A {
		for l in &self.log {
			println!("{l}");
		}
		&self.value
	}
}

impl<A> Functor<A> for Logging<A> {
	type Map<B> = Logging<B>;

	/// Applies the function `f` to the value while keeping the logs intact.
	fn map<B, F: FnOnce(A) -> B>(self, f: F) -> Self::Map<B> {
		Logging {
			value: f(self.value),
			log: self.log,
		}
	}
}

impl<A> Applicative<A> for Logging<A> {
	type Apply<B> = Logging<B>;

	/// Creates a new `Logging` with the `value` and an empty log.
	fn pure(value: A) -> Self {
		Logging { value, log: Vec::new() }
	}

	/// Uses the `value` of `f` as a function for mapping the `value` of
	/// `self`. The logs of `f` are appended to the logs of `self`.
	fn ap<B, F: FnOnce(A) -> B>(self, mut f: Self::Apply<F>) -> Self::Apply<B> {
		let mut next = self.map(f.value);
		next.log.append(&mut f.log);
		next
	}
}

impl<A> Monad<A> for Logging<A> {
	type Bind<B> = Logging<B>;

	/// Applies the function `f` to the value, making sure the logs are a
	/// continuation of the logs of `self`.
	fn bind<B, F: FnOnce(A) -> Self::Bind<B>>(mut self, f: F) -> Self::Bind<B> {
		let mut next = f(self.value);
		self.log.append(&mut next.log);
		next.log = self.log;
		next
	}
}
