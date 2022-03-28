use crate::ref_table::RefTable;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::mem::block::OwnedMemoryBlock;
use crate::mem::{Heap, HeapRegion};
use crate::ptr::GcPtr;
use crate::trace::{AnnotatedMixedHeap, Trace};
#[cfg(feature = "allocator_api")]
use std::alloc::{Allocator, Global};
use std::cell::UnsafeCell;

pub struct VirtualMachine<T, #[cfg(feature = "allocator_api")] A: Allocator = Global> {
    ref_table: Arc<RefTable<T>>,
    #[cfg(feature = "allocator_api")]
    allocator: A,
}

impl<T> VirtualMachine<T> {
    pub fn make_allocator(&self) -> ThreadAllocator<T> {
        ThreadAllocator {
            tlab: todo!(),
            ref_table: self.ref_table.clone(),
            lock_record: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

/// An Allocator<'heap, T> lives for the duration of the heap and acts as a reference to the TLAB
pub struct ThreadAllocator<'heap, T> {
    tlab: UnsafeCell<HeapRegion<OwnedMemoryBlock, AnnotatedMixedHeap>>,
    ref_table: Arc<RefTable<T>>,
    lock_record: Vec<usize>,
    _phantom: PhantomData<&'heap mut T>,
}

impl<'heap, T: Trace> ThreadAllocator<'heap, T> {
    pub fn allocate<A>(&self, value: T) -> Result<GcPtr<T>, T> {
        let direct = unsafe { (&mut *self.tlab.get()).try_push_to_heap(value)? };
        let indirect = self.ref_table.claim_slot().assign(direct);
        todo!()
    }
}
