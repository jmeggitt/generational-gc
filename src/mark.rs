use bitflags::bitflags;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub trait MarkWord: Default {
    /// Get if mark is currently set
    fn is_marked(&self) -> bool;

    /// Enable the mark on this object and return if it was set prior
    fn set_mark(&self) -> bool;

    /// Remove mark
    fn unmark(&self);
}

bitflags! {
    struct TestMarkBits: u64 {
        const MARK_BIT = 0x0000_0000_0000_0001;
    }
}

/// A test mark word for use while flushing out the rest of the code. To keep things simple, this
/// will not actually be the size of a word in memory.
#[repr(transparent)]
#[derive(Default, Debug)]
pub struct TestMark {
    mark: AtomicU64,
}

impl MarkWord for TestMark {
    fn is_marked(&self) -> bool {
        let bits = TestMarkBits::from_bits_truncate(self.mark.load(Ordering::SeqCst));
        bits.contains(TestMarkBits::MARK_BIT)
    }

    fn set_mark(&self) -> bool {
        self.mark
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |bits| {
                if bits & TestMarkBits::MARK_BIT.bits == TestMarkBits::MARK_BIT.bits {
                    return None;
                }

                Some(bits | TestMarkBits::MARK_BIT.bits)
            })
            .is_err()
    }

    fn unmark(&self) {
        self.mark
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |bits| {
                Some(bits & !TestMarkBits::MARK_BIT.bits)
            });
    }
}

bitflags! {
    struct HotspotMarkBits: usize {
        // Areas of
        const LOCK = 0b0000_0011;
        const AGE  = 0b0011_1100;
        const PTR  = !Self::LOCK.bits;
        const HASH = !(Self::LOCK.bits | Self::AGE.bits);

        // States
        const LOCKED    = 0b00;
        const UNLOCKED  = 0b01;
        const MONITOR   = 0b10;
        const MARKED    = 0b11;
        const INFLATING = 0;
    }
}

impl HotspotMarkBits {
    pub fn inflating(self) -> bool {
        self == Self::INFLATING
    }

    pub fn inflate(self) -> Self {
        Self::INFLATING
    }

    pub fn ptr(self) -> *mut () {
        self.intersection(Self::PTR).bits as *mut ()
    }

    pub const fn min_alignment() -> usize {
        let usage = (Self::LOCK.bits | Self::AGE.bits);
        usage.next_power_of_two()
    }
}

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct HotspotMark {
    mark: AtomicUsize,
}

impl HotspotMark {
    pub fn lock(&self, thread_ptr: NonNull<()>) {
        let mut prev = self.mark.load(Ordering::SeqCst);
        // loop {
        //
        //
        //     match self.mark.compare_exchange_weak() {
        //
        //     }
        // }
    }
}

// pub struct Mark {
//
// }
