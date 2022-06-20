use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct OneLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>
}

impl<T> OneLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data)
        }
    }

    pub fn lock(&self) -> OneLockGuard<T> {
        let order = Ordering::Relaxed;
        if self.lock.compare_exchange(false, true, order, order).is_err() {
            panic!("did not expect anyone to hold on to this lock")
        }

        OneLockGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() }
        }
    }
}

unsafe impl<T> Sync for OneLock<T> { }

pub struct OneLockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T
}

impl<'a, T> Deref for OneLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> DerefMut for OneLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T> Drop for OneLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Relaxed);
    }
}
