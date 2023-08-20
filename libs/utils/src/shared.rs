use std::{
    borrow, fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{self, fence, Ordering},
};

const MAX_REFCOUNT: u8 = u8::MAX;

/// A wrapper type providing shared ownership over a value of type `T`, stored on the heap.
///
/// `Shared<T>` allows multiple owners to have shared, read-only access to the same value,
/// ensuring thread-safety through synchronization.
pub struct Shared<T: ?Sized> {
    ptr: NonNull<ArcInner<T>>,
    phantom: PhantomData<Box<T>>,
}

unsafe impl<T: ?Sized + Sync + Send> Send for Shared<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for Shared<T> {}

impl<T> Shared<T> {
    pub fn new(data: T) -> Shared<T> {
        let inner = Box::new(ArcInner {
            data,
            counter: atomic::AtomicU8::new(1),
        });

        Shared {
            // Safety: box is always not null
            ptr: unsafe { NonNull::new_unchecked(Box::leak(inner)) },
            phantom: PhantomData,
        }
    }

    /// Gives you a pointer to the data. The reference count stays the same and
    /// the [`Shared<T>`] isn't used up. The pointer stays valid as long as there
    /// are strong references to the [`Shared<T>`].
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        // SAFETY: ptr is valid, as self is a valid instance of [`Shared<T>`]
        self.ptr.as_ptr() as *const T
    }
}

impl<T: ?Sized> Shared<T> {
    fn inner(&self) -> &ArcInner<T> {
        // SAFETY: inner is protected by counter, it will not get released unless drop
        // of the last owner get called.
        unsafe { self.ptr.as_ref() }
    }

    /// Returns the number of [`Shared`]s that point to the same allocation.
    pub fn strong_count(&self) -> usize {
        self.inner().counter.load(Ordering::Relaxed) as usize
    }

    /// Returns `true` if there are no other [`Shared`]s that point to the same allocation.
    pub fn is_unique(&self) -> bool {
        self.strong_count() == 1
    }

    unsafe fn drop_slow(&mut self) {
        let _ = Box::from_raw(self.ptr.as_ptr());
    }
}

fn drop_and_panic<T>(ptr: NonNull<ArcInner<T>>) {
    drop(Shared {
        ptr,
        phantom: PhantomData,
    });

    panic!("reference counter overflow");
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        let old_size = self.inner().counter.fetch_add(1, Ordering::Relaxed);
        if old_size > MAX_REFCOUNT {
            drop_and_panic(self.ptr);
        }

        Self {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    #[inline]
    fn drop(&mut self) {
        if self.inner().counter.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        fence(Ordering::Acquire);

        // SAFETY: this is the last owner of the ptr, it is safe to drop data
        unsafe { self.drop_slow() };
    }
}

#[repr(C)]
struct ArcInner<T: ?Sized> {
    counter: atomic::AtomicU8,
    data: T,
}

impl<T: ?Sized + Eq> Eq for Shared<T> {}

impl<T: ?Sized + PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other)
    }
}

impl<T: ?Sized + PartialOrd> PartialOrd for Shared<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }

    fn lt(&self, other: &Self) -> bool {
        *(*self) < *(*other)
    }

    fn le(&self, other: &Self) -> bool {
        *(*self) <= *(*other)
    }

    fn gt(&self, other: &Self) -> bool {
        *(*self) > *(*other)
    }

    fn ge(&self, other: &Self) -> bool {
        *(*self) >= *(*other)
    }
}

impl<T: ?Sized + Ord> Ord for Shared<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<T: ?Sized + Hash> Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized> fmt::Pointer for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(&**self as *const T), f)
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Shared<T> {
        Shared::new(Default::default())
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Shared::new(value)
    }
}

impl<T: ?Sized> AsRef<T> for Shared<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> borrow::Borrow<T> for Shared<T> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized> Unpin for Shared<T> {}

impl<T: ?Sized> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared() {
        let shared = Shared::new(1);
        drop(shared)
    }

    #[test]
    fn test_clone() {
        let shared = Shared::new(1);
        let shared2 = shared.clone();

        assert_eq!(shared, shared2);

        drop(shared);
        drop(shared2);
    }

    #[test]
    fn test_multithread() {
        use std::sync::Arc;
        use std::thread;

        let shared = Arc::new(Shared::new(1));

        let mut threads = Vec::new();
        for _ in 0..10 {
            let shared = shared.clone();
            threads.push(thread::spawn(move || {
                assert_eq!(*shared, 1.into());
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }

        assert_eq!(shared.strong_count(), 1);
    }
}
