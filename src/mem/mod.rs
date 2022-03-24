use crate::mem::block::AllocationBlock;
use crate::trace::{AnnotatedMixedHeap, HeapObjectLayout, HeapObjectSetup};
use std::alloc::Layout;
use std::marker::PhantomData;
use std::mem::{align_of, MaybeUninit};
use std::ptr::NonNull;

pub mod block;

pub trait Heap<T>: VisitHeap {
    /// Returns a direct pointer to the uninitialized data if the allocation was successful.
    /// Otherwise None will be returned to indicate allocation failed.
    fn try_alloc_uninit(&mut self) -> Option<NonNull<MaybeUninit<T>>>;

    /// Create an initialized object on the heap by pushing a value. If space can not be allocated,
    /// the value is returned to the caller.
    fn try_push_to_heap(&mut self, value: T) -> Result<NonNull<T>, T> {
        match self.try_alloc_uninit() {
            None => Err(value),
            Some(mut ptr) => unsafe {
                *ptr.as_mut().as_mut_ptr() = value;
                Ok(ptr.cast())
            },
        }
    }
}

pub trait VisitHeap {
    type Entry: Into<NonNull<()>>;
    type EntryIter: IntoIterator<Item = Self::Entry>;

    fn iter_entries(&self) -> Self::EntryIter;
}

pub struct HeapRegion<R, L = AnnotatedMixedHeap> {
    region: R,
    remaining: NonNull<u8>,
    objects: Vec<NonNull<()>>,
    _phantom: PhantomData<L>,
}

impl<R: AllocationBlock, L> From<R> for HeapRegion<R, L> {
    fn from(region: R) -> Self {
        HeapRegion {
            remaining: region.start(),
            region,
            objects: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<R, L> HeapRegion<R, L> {
    /// Completely arbitrary approach in an attempt to make sure contents are aligned to the the
    /// word size
    pub const fn min_alignment() -> usize {
        align_of::<*mut ()>()
    }
}

impl<R: AllocationBlock, L> HeapRegion<R, L> {
    pub fn remaining_space(&self) -> usize {
        self.region.len()
            - (self.remaining.as_ptr() as usize - self.region.start().as_ptr() as usize)
    }

    fn offset_for_align(ptr: usize, align: usize) -> usize {
        if ptr % align == 0 {
            return 0;
        }

        align - (ptr % align)
    }

    /// Allocate a new object within this heap
    pub fn alloc_layout(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let layout = layout.align_to(Self::min_alignment()).ok()?.pad_to_align();
        let padding = Self::offset_for_align(self.remaining.as_ptr() as usize, layout.align());

        if layout.size() + padding > self.remaining_space() {
            return None;
        }

        let target = self.remaining.as_ptr() as usize + padding;
        self.remaining = NonNull::new((target + layout.size()) as *mut _).unwrap();

        Some(NonNull::new(target as *mut _).unwrap())
    }
}

impl<R: AllocationBlock, L: HeapObjectLayout> HeapRegion<R, L> {
    pub fn alloc<T>(&mut self) -> Option<NonNull<T>>
    where
        L: HeapObjectSetup<T>,
    {
        let layout = L::wrap_layout(Layout::new::<T>());
        let allocated = self.alloc_layout(layout)?;

        unsafe { Some(L::init_object(allocated, layout)) }
    }
}
