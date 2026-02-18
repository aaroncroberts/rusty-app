/// Connection pool utilities for database adapters
///
/// This module provides connection pooling abstractions to support
/// adapters that require mutable access to their clients.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

/// A thread-safe, async-compatible connection pool wrapper
///
/// This provides interior mutability for database clients that require
/// mutable access while maintaining a &self API for the DatabaseAdapter trait.
pub struct Pool<T> {
    inner: Arc<Mutex<T>>,
}

// Manual Clone implementation that doesn't require T: Clone
// We can clone the Arc even if T isn't cloneable
impl<T> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Pool<T> {
    /// Create a new pool wrapping the given client
    pub fn new(client: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(client)),
        }
    }

    /// Get exclusive access to the pooled client
    ///
    /// Returns a guard that derefs to the client. The lock is held
    /// until the guard is dropped.
    pub async fn lock(&self) -> PoolGuard<'_, T> {
        PoolGuard {
            guard: self.inner.lock().await,
        }
    }
}

/// A guard providing exclusive access to a pooled client
///
/// This guard implements `Deref` and `DerefMut`, allowing you to
/// call methods on the underlying client directly.
pub struct PoolGuard<'a, T> {
    guard: MutexGuard<'a, T>,
}

impl<'a, T> Deref for PoolGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<'a, T> DerefMut for PoolGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_lock_and_modify() {
        let pool = Pool::new(42i32);

        {
            let mut guard = pool.lock().await;
            *guard += 1;
        }

        let guard = pool.lock().await;
        assert_eq!(*guard, 43);
    }

    #[tokio::test]
    async fn test_pool_with_string() {
        let pool = Pool::new(String::from("hello"));

        {
            let mut guard = pool.lock().await;
            guard.push_str(" world");
        }

        let guard = pool.lock().await;
        assert_eq!(*guard, "hello world");
    }

    #[tokio::test]
    async fn test_pool_clone() {
        let pool1 = Pool::new(100i32);
        let pool2 = pool1.clone();

        {
            let mut guard = pool1.lock().await;
            *guard += 1;
        }

        let guard = pool2.lock().await;
        assert_eq!(*guard, 101);
    }

    #[tokio::test]
    async fn test_pool_deref() {
        let pool = Pool::new(vec![1, 2, 3]);

        {
            let mut guard = pool.lock().await;
            guard.push(4);
            assert_eq!(guard.len(), 4);
        }

        let guard = pool.lock().await;
        assert_eq!(*guard, vec![1, 2, 3, 4]);
    }
}
