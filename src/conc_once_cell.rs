use std::mem;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::sized_box::SizedBox;

pub struct ConcurrentOnceCell<T> {
    ptr: AtomicPtr<T>,
}

impl<T> ConcurrentOnceCell<T> {
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
        }
    }

    pub fn is_init(&self) -> bool {
        !self.ptr.load(Ordering::Acquire).is_null()
    }

    pub fn try_init(&self, val: T) -> Result<(), T> {
        let mut sized = SizedBox::new(val);
        let ptr = sized.as_mut() as *mut T;
        match self
            .ptr
            .compare_exchange(null_mut(), ptr, Ordering::Release, Ordering::Relaxed)
        {
            Ok(_) => {
                mem::forget(sized);
                Ok(())
            }
            Err(_) => Err(sized.into_inner()),
        }
    }

    pub fn get(&self) -> Option<&T> {
        unsafe { self.ptr.load(Ordering::Acquire).as_ref() }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> &T {
        if let Some(val) = self.get() {
            return val;
        }
        self.try_init(f());
        unsafe { self.get().unwrap_unchecked() }
    }
}

impl<T> Drop for ConcurrentOnceCell<T> {
    fn drop(&mut self) {
        let ptr = *self.ptr.get_mut();
        if !ptr.is_null() {
            unsafe {
                ptr.drop_in_place();
            }
        }
    }
}
