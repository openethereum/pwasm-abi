#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), feature(alloc))]
#![feature(proc_macro)]

#[cfg(not(test))]
extern crate alloc;

#[cfg(not(test))]
use alloc::vec::Vec;

extern crate pwasm_abi;
extern crate pwasm_abi_derive;

use pwasm_abi_derive::legacy_dispatch;

#[legacy_dispatch]
trait TestContract {
	fn baz(&mut self, p1: u32, p2: bool);
}

#[test]
fn smoky() {
	struct TestContractInstance;
	impl TestContract for TestContractInstance {
		fn baz(&mut self, p1: u32, p2: bool) {
			println!("baz");
		}
	}

	let mut endpoint = Endpoint::new(TestContractInstance);
	let result = endpoint.dispatch(Vec::new());

	assert_eq!(result, Vec::new());
}