use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{bounded, Sender, Receiver};

// Task representation for parallel execution
pub struct Task {
    function: Box<dyn FnOnce() + Send>,
}

impl Task {
    pub fn new<F>(function: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Task {
            function: Box::new(function),
        }
    }
}

// Thread pool for parallel execution
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Task>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = bounded(size * 2);
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let task = Task::new(f);
        self.sender.as_ref().unwrap().send(task).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let task = match receiver.lock().unwrap().recv() {
                Ok(task) => task,
                Err(_) => break,
            };
            (task.function)();
        });

        Worker {
            _id: id,
            thread: Some(thread),
        }
    }
}

// SIMD Operations Support
#[cfg(target_arch = "x86_64")]
pub mod simd {
    use std::arch::x86_64::*;

    pub unsafe fn vector_add_f32(a: &[f32], b: &[f32], result: &mut [f32]) {
        let len = a.len();
        let mut i = 0;

        // Process 4 elements at a time using SSE
        while i + 4 <= len {
            let va = _mm_loadu_ps(&a[i]);
            let vb = _mm_loadu_ps(&b[i]);
            let sum = _mm_add_ps(va, vb);
            _mm_storeu_ps(&mut result[i], sum);
            i += 4;
        }

        // Handle remaining elements
        while i < len {
            result[i] = a[i] + b[i];
            i += 1;
        }
    }

    pub unsafe fn vector_multiply_f32(a: &[f32], b: &[f32], result: &mut [f32]) {
        let len = a.len();
        let mut i = 0;

        while i + 4 <= len {
            let va = _mm_loadu_ps(&a[i]);
            let vb = _mm_loadu_ps(&b[i]);
            let product = _mm_mul_ps(va, vb);
            _mm_storeu_ps(&mut result[i], product);
            i += 4;
        }

        while i < len {
            result[i] = a[i] * b[i];
            i += 1;
        }
    }
}

// Memory Management
pub mod memory {
    use std::alloc::{alloc, dealloc, Layout};
    use std::ptr::NonNull;

    pub struct Arena {
        ptr: NonNull<u8>,
        size: usize,
        used: usize,
    }

    impl Arena {
        pub fn new(size: usize) -> Self {
            unsafe {
                let layout = Layout::from_size_align_unchecked(size, 8);
                let ptr = NonNull::new(alloc(layout)).expect("allocation failed");
                Arena {
                    ptr,
                    size,
                    used: 0,
                }
            }
        }

        pub fn allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
            let aligned_used = (self.used + align - 1) & !(align - 1);
            if aligned_used + size <= self.size {
                let ptr = unsafe {
                    NonNull::new_unchecked(self.ptr.as_ptr().add(aligned_used))
                };
                self.used = aligned_used + size;
                Some(ptr)
            } else {
                None
            }
        }
    }

    impl Drop for Arena {
        fn drop(&mut self) {
            unsafe {
                dealloc(
                    self.ptr.as_ptr(),
                    Layout::from_size_align_unchecked(self.size, 8),
                );
            }
        }
    }
}

// Error Handling
#[derive(Debug)]
pub enum RuntimeError {
    MemoryAllocationError,
    ThreadPoolError(String),
    SIMDError(String),
    IndexOutOfBounds { index: usize, len: usize },
}

impl std::error::Error for RuntimeError {}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RuntimeError::MemoryAllocationError => write!(f, "Memory allocation failed"),
            RuntimeError::ThreadPoolError(msg) => write!(f, "Thread pool error: {}", msg),
            RuntimeError::SIMDError(msg) => write!(f, "SIMD operation error: {}", msg),
            RuntimeError::IndexOutOfBounds { index, len } => {
                write!(f, "Index {} out of bounds for length {}", index, len)
            }
        }
    }
}
