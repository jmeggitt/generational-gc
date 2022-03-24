use std::ptr::NonNull;
use std::alloc::{GlobalAlloc, Layout, System};

#[cfg(feature = "allocator_api")]
use std::alloc::{Allocator, Global, AllocError};

pub unsafe trait AllocationBlock {
    fn start(&self) -> NonNull<u8>;
    fn len(&self) -> usize;
}

pub struct OwnedMemoryBlock<#[cfg(feature = "allocator_api")] A: Allocator = Global> {
    layout: Layout,
    ptr: NonNull<u8>,
    #[cfg(feature = "allocator_api")]
    allocator: A,
}

unsafe impl AllocationBlock for OwnedMemoryBlock {
    fn start(&self) -> NonNull<u8> {
        self.ptr
    }

    fn len(&self) -> usize {
        self.layout.size()
    }
}

#[cfg(not(feature = "allocator_api"))]
impl OwnedMemoryBlock {
    pub fn new(layout: Layout) -> Self {
        OwnedMemoryBlock {
            layout,
            ptr: NonNull::new(unsafe { System.alloc(layout) })
                .expect("Failed to allocate memory for OwnedMemoryBlock")
        }
    }
}

#[cfg(feature = "allocator_api")]
impl<A: Allocator + Default> OwnedMemoryBlock<A> {
    pub fn new(layout: Layout) -> Self {
        Self::new_in(layout, A::default())
            .expect("Failed to allocate memory for OwnedMemoryBlock")
    }
}

#[cfg(feature = "allocator_api")]
impl<A: Allocator> OwnedMemoryBlock<A> {
    pub fn new_in(layout: Layout, allocator: A) -> Result<Self, AllocError> {
        Ok(OwnedMemoryBlock {
            layout,
            ptr: allocator.allocate(layout)?.cast(),
            allocator,
        })
    }
}

#[cfg(feature = "allocator_api")]
impl<A: Allocator> Drop for OwnedMemoryBlock<A> {
    fn drop(&mut self) {
        unsafe { self.allocator.deallocate(self.ptr, self.layout) }
    }
}

#[cfg(not(feature = "allocator_api"))]
impl Drop for OwnedMemoryBlock {
    fn drop(&mut self) {
        unsafe { System.dealloc(self.ptr.as_ptr(), self.layout) }
    }
}

