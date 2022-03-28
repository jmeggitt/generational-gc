use bitflags::bitflags;
use std::hint::spin_loop;
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
        const LOCK   = 0b0000_0011;
        const BIASED = 0b0000_0100;
        const AGE    = 0b0111_1000;
        const PTR  = !Self::LOCK.bits;
        const HASH = !(Self::LOCK.bits | Self::BIASED.bits | Self::AGE.bits);

        // States
        const LOCKED    = 0b00;
        const UNLOCKED  = 0b01;
        const MONITOR   = 0b10;
        const MARKED    = 0b11;
        const INFLATING = 0;
    }
}

// Mark Regions
const LOCK_BITS: usize = 0b0000_0011;
const BIASED_BITS: usize = 0b0000_0100;
const AGE_BITS: usize = 0b0111_1000;

// Lock States
const LOCKED: usize = 0b00;
const UNLOCKED: usize = 0b01;
const MONITOR: usize = 0b10;
const MARKED: usize = 0b11;
const INFLATING: usize = 0;

// #[repr(usize)]
// enum MarkState {
//     Unlocked = 0b01,
//     Locked = 0b00,
//     Monitor = 0b10,
//     Marked = 0b11,
// }

impl HotspotMarkBits {
    // pub fn state(self) -> MarkState {}

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

pub trait LockRecord {
    fn store(&mut self, value: usize) -> *mut usize;
    fn forfeit(&mut self, ptr: *mut usize);
}

#[repr(transparent)]
#[derive(Default, Debug)]
pub struct HotspotMark {
    mark: AtomicUsize,
}

impl HotspotMark {
    pub fn lock<S: LockRecord>(&self, lock_record: &mut S) {
        let mut prev_mark = self.mark.load(Ordering::SeqCst);

        while prev_mark == INFLATING {
            spin_loop();
            prev_mark = self.mark.load(Ordering::SeqCst);
        }

        match prev_mark & LOCK_BITS {
            UNLOCKED => {
                // Store current mark
                let obj_ptr = lock_record.store(prev_mark);
                debug_assert_eq!(
                    obj_ptr as usize % 3,
                    0,
                    "Lock record alignment must be at least 8"
                );

                match self.mark.compare_exchange(
                    prev_mark,
                    obj_ptr as usize,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    // Successfully obtained lock
                    Ok(_) => return,
                    Err(_) => {
                        lock_record.forfeit(obj_ptr);
                    }
                }

                // TODO: Attempt claim block
            }
            LOCKED => {
                // TODO: Inflate Lock
            }
            MONITOR => {
                // TODO: Wait on monitor
            }
            _ => panic!("Multiple heaps may be referencing the same region"),
        };
    }
}

// pub struct Mark {
//
// }
