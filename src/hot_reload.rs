use crate::tree::{A, NODE_COUNT};

#[no_mangle]
pub fn _rosin_reset_alloc() -> Result<(), ()> {
    A.with(|a| a.reset())
}

#[no_mangle]
pub fn _rosin_begin_alloc() {
    A.with(|a| a.begin())
}

#[no_mangle]
pub fn _rosin_end_alloc() {
    A.with(|a| a.end())
}

#[no_mangle]
pub fn _rosin_reset_node_count() {
    NODE_COUNT.with(|c| c.set(0))
}

#[no_mangle]
pub fn _rosin_get_node_count() -> usize {
    NODE_COUNT.with(|c| c.get())
}
