use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::marker::PhantomData;

/// A growable vector implementation with zero-cost abstractions
pub struct Vector<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
    _marker: PhantomData<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Vector {
            ptr: NonNull::dangling(),
            len: 0,
            capacity: 0,
            _marker: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let layout = Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe {
            NonNull::new(alloc(layout) as *mut T)
                .expect("Failed to allocate memory")
        };

        Vector {
            ptr,
            len: 0,
            capacity,
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.len == self.capacity {
            self.grow();
        }
        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe {
                Some(self.ptr.as_ptr().add(self.len).read())
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn grow(&mut self) {
        let new_capacity = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };

        let new_layout = Layout::array::<T>(new_capacity).unwrap();
        let new_ptr = unsafe {
            let ptr = alloc(new_layout) as *mut T;
            if !self.ptr.as_ptr().is_null() {
                std::ptr::copy_nonoverlapping(
                    self.ptr.as_ptr(),
                    ptr,
                    self.len,
                );
                dealloc(
                    self.ptr.as_ptr() as *mut u8,
                    Layout::array::<T>(self.capacity).unwrap(),
                );
            }
            NonNull::new(ptr).expect("Failed to allocate memory")
        };

        self.ptr = new_ptr;
        self.capacity = new_capacity;
    }
}

impl<T> Drop for Vector<T> {
    fn drop(&mut self) {
        if self.capacity > 0 {
            unsafe {
                for i in 0..self.len {
                    self.ptr.as_ptr().add(i).drop_in_place();
                }
                dealloc(
                    self.ptr.as_ptr() as *mut u8,
                    Layout::array::<T>(self.capacity).unwrap(),
                );
            }
        }
    }
}

// Implement common traits
impl<T: Clone> Clone for Vector<T> {
    fn clone(&self) -> Self {
        let mut new_vec = Vector::with_capacity(self.capacity);
        for i in 0..self.len {
            unsafe {
                new_vec.push((*self.ptr.as_ptr().add(i)).clone());
            }
        }
        new_vec
    }
}

impl<T> std::ops::Deref for Vector<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
        }
    }
}

impl<T> std::ops::DerefMut for Vector<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
        }
    }
}
