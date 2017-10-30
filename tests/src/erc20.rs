
mod contract {
	use pwasm_abi_derive::eth_abi;
	use parity_hash::Address;
	use bigint::U256;
	use call;
	use std::collections::HashMap;

	#[cfg(not(test))]
	use alloc::borrow::Cow;
	#[cfg(test)]
	use std::borrow::Cow;

	#[allow(non_snake_case)]
	#[eth_abi(Endpoint, Client)]
	pub trait TokenContract {
		fn ctor(&mut self, total_supply: U256);
		fn balanceOf(&mut self, _owner: Address) -> U256;
		fn transfer(&mut self, _to: Address, _amount: U256) -> bool;
		fn totalSupply(&mut self) -> U256;
	}

	#[derive(Default)]
	pub struct Instance {
		total_supply: U256,
		balances: HashMap<Address, U256>,
	}

	impl TokenContract for Instance {
		fn ctor(&mut self, total_supply: U256) {
			self.total_supply = total_supply;
		}

		fn balanceOf(&mut self, owner: Address) -> U256 {
			self.balances.get(&owner).cloned().unwrap_or(U256::zero())
		}

		fn transfer(&mut self, to: Address, amount: U256) -> bool {
			false
		}

		fn totalSupply(&mut self) -> U256 {
			self.total_supply
		}
	}
}

// balanceOf(0x0)
const SAMPLE1: &'static [u8] = &[
	0x70, 0xa0, 0x82, 0x31,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[test]
fn call1() {
	let mut endpoint = contract::Endpoint::new(contract::Instance::default());
	endpoint.dispatch(SAMPLE1);
}

#[test]
#[should_panic]
fn ctor_empty() {
	let mut endpoint = contract::Endpoint::new(contract::Instance::default());
	endpoint.dispatch_ctor(&vec![]);
}