use crate::mark::MarkWord;
use crate::ptr::DirectObjUnknown;
use crate::trace::HeapObjectLayout;
use std::hint::spin_loop;
use std::mem::size_of;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

pub unsafe trait VisitHeap: Sized {
    type Layout: HeapObjectLayout;
    type EntryIter: IntoIterator<Item = DirectObjUnknown>;

    fn iter_entries(self) -> Self::EntryIter;

    unsafe fn unmark_heap(self) {
        for entry in self.iter_entries() {
            Self::Layout::mark(entry).unmark();
        }
    }
}

/// A counter used for keeping track of the number of entries within a heap region and closing off
/// access before a garbage collection sweep.
#[derive(Debug, Default)]
pub struct AccessCounter {
    counter: AtomicUsize,
}

/// AccessCounter is safe since it only performs atomic operations on its contents
unsafe impl Sync for AccessCounter {}

impl AccessCounter {
    /// The close mask is simply the highest bit
    const CLOSE_MASK: usize = 1usize << (8 * size_of::<usize>() - 1);
    const COUNT_MASK: usize = !Self::CLOSE_MASK;

    pub fn close_counter(&self) -> CloseGuard {
        unsafe {
            self.request_close();
        }
        CloseGuard { inner: self }
    }

    /// Attempts to increment counter or blocks for savepoint so GC can run. increment_or_savepoint
    /// should be prefered over increment when possible, but running overlapping
    /// increment_or_savepoint on a single thread may result in a deadlock. When an overlap may
    /// occur, increment can be used for subsequent calls.
    pub fn increment_or_savepoint(&self) -> IncrementGuard {
        unsafe {
            self.blocking_enter();
        }
        IncrementGuard { inner: self }
    }

    pub fn increment(&self) -> IncrementGuard {
        unsafe {
            self.forced_entry();
        }
        IncrementGuard { inner: self }
    }

    /// Request that this counter be closed. If the counter is already in the process of being
    /// closed, this function will block until it can be closed in favor of this thread.
    pub unsafe fn request_close(&self) {
        let mut prev = self.counter.load(Ordering::SeqCst);
        loop {
            if prev & Self::CLOSE_MASK != 0 {
                spin_loop();
                prev = self.counter.load(Ordering::SeqCst);
                continue;
            }

            let new = prev & Self::CLOSE_MASK;
            match self
                .counter
                .compare_exchange_weak(prev, new, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return,
                Err(e) => prev = e,
            }
        }
    }

    /// Blocks until the counter is completely closed. However, this function will not initiate the
    /// close.
    pub fn block_until_closed(&self) {
        loop {
            match self.counter.load(Ordering::SeqCst) {
                Self::CLOSE_MASK => return,
                _ => spin_loop(),
            }
        }
    }

    /// Released a close request. Should only be called by closer.
    pub unsafe fn release_close_request(&self) {
        self.counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |counter| {
                assert_eq!(counter & Self::CLOSE_MASK, Self::CLOSE_MASK);
                Some(counter & Self::COUNT_MASK)
            });
    }

    /// Attempt to increment the counter to gain entry. Ignores if a thread is attempting to close
    /// the counter and always increments the counter if at least one other thread has yet to leave.
    /// However, if the current count is zero and there is a close request then the close request
    /// must be respected and this function will block until it can be obtained.
    ///
    /// This option is available to prevent deadlocks when two or more items need to enter the
    /// counter at the same time. Using blocking_enter should always be preferred unless there is
    /// an existing entry on that thread which has yet to be released.
    pub unsafe fn forced_entry(&self) {
        let gained_entry = self
            .counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| match value {
                Self::CLOSE_MASK => None,
                x => {
                    let new_count = (x & Self::COUNT_MASK) + 1;
                    debug_assert_eq!(new_count & Self::CLOSE_MASK, 0);

                    Some((x & Self::CLOSE_MASK) | (new_count & Self::COUNT_MASK))
                }
            })
            .is_ok();

        // Default to blocking if entry was not gained
        if !gained_entry {
            self.blocking_enter();
        }
    }

    /// Increments the counter to gain entry. If the counter is being closed, it will block until
    /// it reopens.
    pub unsafe fn blocking_enter(&self) {
        let mut prev = self.counter.load(Ordering::SeqCst);
        loop {
            if prev & Self::CLOSE_MASK != 0 {
                // TODO: Should this be switched to a system which allows the current thread to sleep until it is acquired?
                spin_loop();
                prev = self.counter.load(Ordering::SeqCst);
                continue;
            }

            let new_count = prev + 1;
            debug_assert_eq!(new_count & Self::CLOSE_MASK, 0);
            match self.counter.compare_exchange_weak(
                prev,
                new_count,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return,
                Err(e) => prev = e,
            }
        }
    }

    /// Decrease the counter after finishing work
    pub unsafe fn exit_counter(&self) {
        self.counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                debug_assert!(count & Self::COUNT_MASK > 0);
                Some(count - 1)
            });
    }
}
/// A simple wrapper that ensures that when a counter is incremented, it gets decremented once
/// finished.
pub struct IncrementGuard<'a> {
    inner: &'a AccessCounter,
}

impl<'a> Drop for IncrementGuard<'a> {
    fn drop(&mut self) {
        unsafe { self.inner.exit_counter() }
    }
}

/// A simple wrapper that ensures that when a counter is closed, it gets reopened once dropped.
pub struct CloseGuard<'a> {
    inner: &'a AccessCounter,
}

impl<'a> CloseGuard<'a> {
    /// Blocks until the counter is completely closed
    pub fn block_until_closed(&self) {
        // Defer to inner version
        self.inner.block_until_closed()
    }
}

impl<'a> Drop for CloseGuard<'a> {
    fn drop(&mut self) {
        // #Safety: We know there is an existing close request since CloseGuard was created for one
        unsafe { self.inner.release_close_request() }
    }
}
