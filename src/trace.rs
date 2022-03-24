use std::alloc::Layout;
use std::ptr::NonNull;

pub type TraceContext = ();


pub unsafe trait HeapObjectLayout {
    /// Get the layout of an unknown object on the heap by its pointer
    unsafe fn layout(ptr: NonNull<()>) -> Layout;

    /// Invoke the trace function of an unknown object on the heap
    unsafe fn trace(ptr: NonNull<()>, cxt: &mut TraceContext);

    /// Drop the data of an unknown object on the heap
    #[cfg(feature = "drop_heap")]
    unsafe fn drop(ptr: NonNull<()>);
}


pub trait HeapObjectSetup<T>: HeapObjectLayout {
    fn wrap_layout(data_layout: Layout) -> Layout;

    unsafe fn init_object(ptr: NonNull<u8>, layout: Layout) -> NonNull<T>;
}

pub trait Trace {
    unsafe fn trace(&self, cxt: &mut TraceContext);
}

pub struct AnnotatedMixedHeap;

unsafe impl HeapObjectLayout for AnnotatedMixedHeap {
    unsafe fn layout(ptr: NonNull<()>) -> Layout {
        let annotation = ptr.cast::<HeapAnnotation>().as_ref();
        annotation.layout
    }

    unsafe fn trace(ptr: NonNull<()>, cxt: &mut TraceContext) {
        let annotation = ptr.cast::<HeapAnnotation>().as_ref();
        (annotation.vtable.trace)(ptr, cxt);
    }

    #[cfg(feature = "drop_heap")]
    unsafe fn drop(ptr: NonNull<()>) {
        let annotation = ptr.cast::<HeapAnnotation>().as_ref();
        (annotation.vtable.drop)(ptr, cxt);
    }
}

impl<T: TypedTrace> HeapObjectSetup<T> for AnnotatedMixedHeap {
    fn wrap_layout(_data_layout: Layout) -> Layout {
        Layout::new::<AnnotatedHeapData<T>>()
    }

    unsafe fn init_object(ptr: NonNull<u8>, layout: Layout) -> NonNull<T> {
        let heap = ptr.cast::<AnnotatedHeapData<T>>().as_mut();

        heap.annotation.layout = layout;
        heap.annotation.vtable = T::vtable();

        NonNull::new_unchecked(&mut heap.data as *mut T)
    }
}


trait TypedTrace {
    fn vtable() -> ObjectVTable;

    unsafe fn _trace(ptr: NonNull<()>, cxt: &mut TraceContext);

    unsafe fn _drop(ptr: NonNull<()>);
}

impl<T: Trace> TypedTrace for T {
    fn vtable() -> ObjectVTable {
        ObjectVTable {
            trace: <T as TypedTrace>::_trace,
            #[cfg(feature = "drop_heap")]
            drop: <T as TypedTrace>::_drop,
        }
    }

    unsafe fn _trace(ptr: NonNull<()>, cxt: &mut TraceContext) {
        Trace::trace(&ptr.cast::<AnnotatedHeapData<T>>().as_ref().data, cxt)
    }

    unsafe fn _drop(ptr: NonNull<()>) {
        std::ptr::drop_in_place(&mut ptr.cast::<AnnotatedHeapData<T>>().as_mut().data as *mut T)
    }
}


#[repr(C)]
struct HeapAnnotation {
    layout: Layout,
    vtable: ObjectVTable,
}

#[repr(C)]
struct ObjectVTable {
    trace: unsafe fn(ptr: NonNull<()>, cxt: &mut TraceContext),
    #[cfg(feature = "drop_heap")]
    drop: unsafe fn(ptr: NonNull<()>),
}

#[repr(C)]
pub struct AnnotatedHeapData<T> {
    annotation: HeapAnnotation,
    data: T,
}






