use std::fmt::{Formatter, Pointer};
use std::ops::Deref;
use std::ptr::NonNull;

/// Placeholder so it can be swapped out later with a struct if needed
pub type DirectObjPtr<T> = NonNull<T>;

/// A direct pointer to an object of unknown type
pub type DirectObjUnknown = DirectObjPtr<()>;

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct GcPtr<T: ?Sized> {
    ptr: NonNull<DirectObjPtr<T>>,
}

impl<T: ?Sized> GcPtr<T> {
    /// Get the direct pointer to this object in memory. This pointer may shift during garbage
    /// collection.
    pub fn direct_ptr(&self) -> *mut T {
        unsafe { (&*self.ptr.as_ptr()).as_ptr() }
    }

    // pub unsafe fn as_ref_unchecked(&self) -> &T {
    //     &*(*self.ptr.as_ptr()).as_ptr()
    // }
    //
    // pub unsafe fn as_mut_unchecked(&self) -> &mut T {
    //     &mut *(*self.ptr.as_ptr()).as_ptr()
    // }
}

impl<T: ?Sized> Pointer for GcPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.ptr, f)
    }
}


/// I'm not happy with how this looks, but it should be completely safe. To access data it will need
/// to be used with a `ThreadAllocator` to ensure that it meets the lifetime requirements and to
/// verify that the pointer it uses matches the specified vm. It should correctly produce
/// an error when used against a different vm, but will continue to work for any `ThreadAllocator`
/// on the vm it was allocated for. It will also be able to deny objects that have since been
/// deleted.
pub struct SafeGcPtr<T: ?Sized> {
    vm: NonNull<()>,
    ptr: NonNull<T>,
    generation: u64,
}



// TODO: Implement generational indices for weak GC pointers
// pub struct WeakGcPtr<T> {
//     ptr: GcPtr<T>,
//     generation: u64,
// }
