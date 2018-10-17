#![feature(extern_prelude)]
#![cfg_attr(not(feature="test"), no_std)]
#![cfg_attr(not(feature="test"), feature(alloc))]
#![feature(use_extern_macros)]
#![feature(proc_macro_hygiene)]
#![cfg(test)]

// extern crate compiletest_rs as compiletest;

extern crate pwasm_std;
extern crate pwasm_ethereum;
extern crate pwasm_test;
extern crate pwasm_abi;
extern crate pwasm_abi_derive;

mod erc20;
mod arrays;
mod trivia;
mod payable;
mod multiple_return;
mod detail;
