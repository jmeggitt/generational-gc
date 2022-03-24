use std::cell::UnsafeCell;
use std::convert::TryInto;
use std::mem::size_of;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use parking_lot::Mutex;
use crate::collect::DirectObjPtr;


#[derive(Copy, Clone)]
union ObjectOrNextEmpty<T: ?Sized> {
    object: DirectObjPtr<T>,
    next_empty: Option<NonNull<Self>>,
}

pub struct OpenRefSlot<T: ?Sized> {
    wrapped: NonNull<ObjectOrNextEmpty<T>>,
}

impl<T: ?Sized> OpenRefSlot<T> {
    pub fn assign(mut self, ptr: DirectObjPtr<T>) -> NonNull<DirectObjPtr<T>> {
        unsafe {
            self.wrapped.as_mut().object = ptr;
            self.wrapped.cast()
        }
    }
}

#[test]
#[cfg(test)]
fn check_ptr_size() {
    // Verify that ObjectOrNextEmpty<T> is the size of a pointer
    // If it isn't it won't break anything, but it would be memory inefficient
    assert_eq!(size_of::<ObjectOrNextEmpty<()>>(), size_of::<*mut ()>());
}

impl<T: ?Sized> Default for ObjectOrNextEmpty<T> {
    fn default() -> Self {
        ObjectOrNextEmpty {
            next_empty: None,
        }
    }
}


/// Size of RefTableBlock. Highly arbitrary, but attempts to be a multiple/factor of the page size.
const BLOCK_SIZE: usize = 4096;

#[repr(transparent)]
struct RefTableBlock<T: ?Sized> {
    ptr: Box<[ObjectOrNextEmpty<T>; BLOCK_SIZE]>,
}

impl<T: ?Sized> RefTableBlock<T> {
    pub fn new() -> Self {
        let mut vec = Vec::with_capacity(BLOCK_SIZE);
        vec.resize_with(BLOCK_SIZE, || ObjectOrNextEmpty { next_empty: None });

        for idx in 0..BLOCK_SIZE - 1 {
            vec[idx] = ObjectOrNextEmpty {
                next_empty: Some(NonNull::new(&vec[idx + 1] as *const _ as *mut _).unwrap())
            }
        }

        match vec.into_boxed_slice().try_into() {
            Ok(ptr) => RefTableBlock { ptr },
            Err(_) => unreachable!(),
        }
    }

    fn add_to_chain(&mut self, new_end: NonNull<ObjectOrNextEmpty<T>>) -> *mut ObjectOrNextEmpty<T> {
        self.ptr[BLOCK_SIZE - 1] = ObjectOrNextEmpty {
            next_empty: Some(new_end),
        };

        &mut self.ptr[0] as *mut _
    }
}

pub struct RefTable<T: ?Sized> {
    blocks: Mutex<Vec<RefTableBlock<T>>>,
    empty: AtomicPtr<ObjectOrNextEmpty<T>>,
}

impl<T> RefTable<T> {

    pub fn new() -> Self {
        let first_block = RefTableBlock::new();
        let empty_ptr = AtomicPtr::new(&first_block.ptr[0] as *const _ as *mut _);

        RefTable {
            blocks: Mutex::new(vec![first_block]),
            empty: empty_ptr,
        }
    }

    /// Frees positions in the reference table for reuse.
    ///
    /// # Safety
    /// Items in the iterator must have been provided by this RefTable. Items also must not be in
    /// use. Using any of the pointers provided after calling this method is undefined behavior.
    pub unsafe fn free_slots<I: Iterator<Item=NonNull<DirectObjPtr<T>>>>(&self, slots: I) {
        let mut slots = slots.map(|x| x.cast::<ObjectOrNextEmpty<T>>());

        let first_slot = match slots.next() {
            Some(v) => v,
            None => return,
        };

        let mut last_slot = first_slot;

        for slot in slots {
            unsafe {
                last_slot.as_mut().next_empty = Some(slot);
                last_slot = slot;
            }
        }

        // Fit the new chain into the empty items list
        loop {
            let current = self.empty.load(Ordering::SeqCst);

            unsafe {
                last_slot.as_mut().next_empty = Some(NonNull::new(current).unwrap());
            }

            if self.empty.compare_exchange(current, first_slot.as_ptr(), Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                return
            }
        }
    }

    pub fn claim_slot(&self) -> OpenRefSlot<T> {
        // Loop until we successfully update the empty index
        loop {
            let current = self.empty.load(Ordering::SeqCst);

            match unsafe { (&*current).next_empty } {
                Some(next) => {
                    if self.empty.compare_exchange(current, next.as_ptr(), Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                        return OpenRefSlot {
                            wrapped: NonNull::new(current).unwrap(),
                        };
                    }
                }
                None => self.attempt_add_new_ref_block(),
            }
        }
    }


    fn attempt_add_new_ref_block(&self) {
        let mut guard = match self.blocks.try_lock() {
            Some(guard) => guard,
            None => return,
        };

        let mut new_block = RefTableBlock::new();

        loop {
            let previous = self.empty.load(Ordering::SeqCst);
            let root = NonNull::new(previous as *mut _).unwrap();

            let new_root = new_block.add_to_chain(root);

            // This should not fail, but loop just in case
            if let Ok(_) = self.empty.compare_exchange(previous, new_root, Ordering::SeqCst, Ordering::SeqCst) {
                guard.push(new_block);
                return;
            }
        }
    }

}
