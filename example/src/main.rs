use mrust::*;

fn main() {
	let success = returning_test();
	println!("Returning test:\t{success:?}");
	let failure = failing_test();
	println!("Failing test:\t{failure:?}");
}

#[monadic]
fn returning_test() -> Option<i32> {
	let a = some_thing()?;
	let b = 2;
	ret(a + b)
}

#[monadic]
fn failing_test() -> Option<i32> {
	let a = some_thing()?;
	let b = no_thing()?;
	ret(a + b)
}

fn some_thing() -> Option<i32> {
	ret(2)
}

fn no_thing() -> Option<i32> {
	None
}
