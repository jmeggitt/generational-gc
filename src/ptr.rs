use std::ptr::NonNull;
use crate::collect::DirectObjPtr;
use crate::header::Header;

#[repr(transparent)]
pub struct GcPtr<T> {
    ptr: NonNull<DirectObjPtr<T>>,
}

// TODO: Implement generational indices for weak GC pointers
pub struct WeakGcPtr<T> {
    ptr: GcPtr<T>,
    generation: u64,
}





