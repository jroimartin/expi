//! Mutual exclusion primitives useful for protecting shared data.
//!
//! The algorithms used under the hood have been taken from the paper
//! [Algorithms for Scalable Synchronization on Shared Memory
//! Multiprocessors][1].
//!
//! [1]: https://web.mit.edu/6.173/www/currentsemester/readings/R06-scalable-synchronization-1991.pdf

#![no_std]

use core::cell::UnsafeCell;
use core::hint;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

/// A mutex based on a ticket lock.
///
/// This type of spin lock ensures FIFO service by granting the lock to
/// processors in the same order in which they first requested it. A ticket
/// lock is fair in a strong sense; it eliminates the possibility of
/// starvation.
pub struct TicketMutex<T> {
    /// Number of requests to adquire the lock.
    next_ticket: AtomicUsize,

    /// Number of times the lock has been released.
    now_serving: AtomicUsize,

    /// Protected data.
    data: UnsafeCell<T>,
}

/// An RAII implementation of a "scoped lock" on a mutex. When this structure
/// is dropped, the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// [`Deref`] and [`DerefMut`] implementations.
///
/// This structure is created by the [`TicketMutex::lock`] method.
pub struct TicketMutexGuard<'a, T> {
    /// The mutex that created this [`TicketMutexGuard`] on lock.
    mutex: &'a TicketMutex<T>,
}

impl<T> TicketMutex<T> {
    /// Returns a new [`TicketMutex`] protecting `data`.
    pub const fn new(data: T) -> TicketMutex<T> {
        TicketMutex {
            next_ticket: AtomicUsize::new(0),
            now_serving: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// Locks the mutex and returns a [`TicketMutexGuard`] that grants
    /// exclusive access to the protected data until it is dropped.
    pub fn lock(&self) -> TicketMutexGuard<T> {
        let my_ticket = self.next_ticket.fetch_add(1, Ordering::SeqCst);
        while my_ticket != self.now_serving.load(Ordering::SeqCst) {
            hint::spin_loop()
        }
        TicketMutexGuard { mutex: self }
    }
}

unsafe impl<T: Send> Send for TicketMutex<T> {}
unsafe impl<T: Send> Sync for TicketMutex<T> {}

impl<T> Deref for TicketMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for TicketMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for TicketMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.now_serving.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutex_lock() {
        let mutex = TicketMutex::new(0);

        let mut x = mutex.lock();
        *x += 1;
        drop(x);

        let mut x = mutex.lock();
        *x += 1;
        drop(x);

        let x = mutex.lock();
        assert_eq!(*x, 2);
    }
}
