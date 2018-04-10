#![allow(dead_code)]
/// The eth_abi marco generates code uses
/// `bigint`, `parity_hash`, `pwasm_ethereum` and `pwasm_abi` crates
/// The following code demonstrates that user dont have to import these crates
/// unless he doesn't use one of those directly

use pwasm_abi_derive::eth_abi;

#[eth_abi(Endpoint, Client)]
pub trait Contract {
	fn constructor(&mut self, _p: bool);

	fn baz(&mut self, _p1: u32, _p2: bool);
	fn boo(&mut self, _arg: u32) -> u32;
	fn sam(&mut self, _p1: Vec<u8>);

	#[event]
	fn baz_fired(&mut self, indexed_p1: u32, p2: u32);
}

struct TestContractInstance;

impl Contract for TestContractInstance {
	fn constructor(&mut self, _p1: bool) {
	}
	fn baz(&mut self, _p1: u32, _p2: bool) {
	}
	fn boo(&mut self, _arg: u32) -> u32 {
		0
	}
	fn sam(&mut self, _p1: Vec<u8>) {
	}
}

#[test]
fn inner_externs_test() {
	Endpoint::new(TestContractInstance{});
}
