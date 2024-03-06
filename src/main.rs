use rdkit_rust_sys;

fn main() {
    unsafe { rdkit_rust_sys::rdkit_rdprops_del(rdkit_rust_sys::rdkit_rdprops_new()) };
}
