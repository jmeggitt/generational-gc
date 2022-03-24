use crate::ref_table::RefTable;
use std::marker::PhantomData;
use std::sync::Arc;

#[cfg(feature = "allocator_api")]
use std::alloc::{Allocator, Global};

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
