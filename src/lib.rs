#![cfg_attr(feature = "allocator_api", feature(allocator_api))]

pub mod alloc;
pub mod ptr;
mod header;
mod collect;
mod util;
pub mod ref_table;
pub mod trace;
mod mem;

use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static FOO: RefCell<u32> = RefCell::new(1)
}
