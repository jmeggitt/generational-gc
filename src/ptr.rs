use std::ptr::NonNull;

/// Placeholder so it can be swapped out later with a struct if needed
pub type DirectObjPtr<T> = NonNull<T>;

#[repr(transparent)]
pub struct GcPtr<T> {
    ptr: NonNull<DirectObjPtr<T>>,
}

// TODO: Implement generational indices for weak GC pointers
pub struct WeakGcPtr<T> {
    ptr: GcPtr<T>,
    generation: u64,
}
