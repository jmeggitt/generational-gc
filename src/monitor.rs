use parking_lot::{Condvar, Mutex};
use std::thread::{current, ThreadId};

#[derive(Default)]
struct ObjectMonitor {
    mutex: Mutex<Option<(ThreadId, u64)>>,
    condvar: Condvar,
}

impl ObjectMonitor {
    fn lock(&self) {
        let current_thread = current().id();
        let mut guard = self.mutex.lock();

        while guard.is_some() {
            // Lock is already held by this thread increment counter and continue
            if guard.unwrap().0 == current_thread {
                guard.unwrap().1 += 1;
                return;
            }

            self.condvar.wait(&mut guard);
        }

        *guard = Some((current().id(), 1));
    }

    fn try_lock(&self) -> bool {
        let mut guard = self.mutex.lock();

        if guard.is_none() {
            *guard = Some((current().id(), 1));
            return true;
        }

        false
    }

    fn unlock(&self) {
        let mut guard = self.mutex.lock();
        let mut break_lock = false;

        if let Some((lock_holder, count)) = &mut *guard {
            if *lock_holder == current().id() {
                *count -= 1;
            }

            if *count == 0 {
                break_lock = true;
            }
        }

        if break_lock {
            *guard = None;
        }

        self.condvar.notify_one();
    }

    fn check_lock(&self) -> bool {
        self.mutex.lock().is_some()
    }
}
