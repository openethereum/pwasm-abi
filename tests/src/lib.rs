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
	fn boo(&mut self, arg: u32) -> u32;
}

#[test]
fn smoky() {
	struct TestContractInstance;
	impl TestContract for TestContractInstance {
		fn baz(&mut self, _p1: u32, _p2: bool) {
			println!("baz");
		}
		fn boo(&mut self, _arg: u32) -> u32 {
			println!("boo");
			0
		}
	}

	let mut endpoint = Endpoint::new(TestContractInstance);
	let result = endpoint.dispatch(Vec::new());

	assert_eq!(result, Vec::new());
}