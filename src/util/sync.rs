use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct SingleLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>
}

impl<T> SingleLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data)
        }
    }

    pub fn lock(&self) -> SingleLockGuard<T> {
        let order = Ordering::Relaxed;
        if self.lock.compare_exchange(false, true, order, order).is_err() {
            panic!("did not expect anyone to hold on to this lock")
        }

        SingleLockGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() }
        }
    }
}

unsafe impl<T> Sync for SingleLock<T> { }

pub struct SingleLockGuard<'a, T> {
    lock: &'a AtomicBool,
    data: &'a mut T
}

impl<'a, T> Deref for SingleLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> DerefMut for SingleLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T> Drop for SingleLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Relaxed);
    }
}
