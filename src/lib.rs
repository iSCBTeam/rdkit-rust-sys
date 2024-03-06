#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::mem::MaybeUninit;

extern crate core;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

impl Default for rdkit_mol_ops_RemoveHsParameters {
	fn default() -> Self {
		let mut params = MaybeUninit::uninit();
		unsafe {
			rdkit_mol_ops_remove_hs_parameters_init_defaults(params.as_mut_ptr());
			params.assume_init()
		}
	}
}
