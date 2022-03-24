use std::marker::PhantomData;
use std::sync::Arc;
use crate::ref_table::RefTable;

#[cfg(feature = "allocator_api")]
use std::alloc::{Allocator, Global, AllocError};
use std::alloc::{GlobalAlloc, Layout, System};
use std::mem::align_of;
use std::ptr::NonNull;
use crate::trace::{HeapObjectLayout, HeapObjectSetup};


pub struct Heap<T, #[cfg(feature = "allocator_api")] A: Allocator = Global> {
    ref_table: Arc<RefTable<T>>,
    #[cfg(feature = "allocator_api")]
    allocator: A,
}

impl<T> Heap<T> {
    pub fn make_allocator(&self) -> ThreadAllocator<T> {
        ThreadAllocator {
            ref_table: self.ref_table.clone(),
            _phantom: PhantomData,
        }
    }
}




/// An Allocator<'heap, T> lives for the duration of the heap and acts as a reference to the TLAB
pub struct ThreadAllocator<'heap, T> {
    ref_table: Arc<RefTable<T>>,
    _phantom: PhantomData<&'heap mut T>,
}

// impl<'heap, T> Allocator<'heap, T> {
//
//     // pub fn new()
//
// }


pub struct ObjectSlot<H, T> {
    header: H,
    data: T,
}


// TODO: Might be a good candidate for replacing with a Vec
#[repr(C, align(4096))]
pub struct ArenaAllocator<const N: usize> {
    arena: [u8; N],
    // TODO: Since N will likely be a multiple of 4096, this usize will likely take an entire page
    position: usize,
}
