use std::io::{self, Write, Read};
use std::fmt;
use crate::stdlib::collections::Vector;

/// Print formatted text to standard output
pub fn print(args: fmt::Arguments) {
    io::stdout().write_fmt(args).unwrap();
}

/// Print formatted text with a newline
pub fn println(args: fmt::Arguments) {
    print(args);
    io::stdout().write_all(b"\n").unwrap();
}

/// Read a line from standard input
pub fn readln() -> String {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}

/// File operations with ownership semantics
pub struct File {
    inner: std::fs::File,
    path: String,
}

impl File {
    pub fn open(path: &str) -> io::Result<Self> {
        Ok(File {
            inner: std::fs::File::open(path)?,
            path: path.to_string(),
        })
    }

    pub fn create(path: &str) -> io::Result<Self> {
        Ok(File {
            inner: std::fs::File::create(path)?,
            path: path.to_string(),
        })
    }

    pub fn read_to_string(&mut self) -> io::Result<String> {
        let mut content = String::new();
        self.inner.read_to_string(&mut content)?;
        Ok(content)
    }

    pub fn read_to_bytes(&mut self) -> io::Result<Vector<u8>> {
        let mut content = Vector::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = self.inner.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            for &byte in &buffer[..bytes_read] {
                content.push(byte);
            }
        }
        
        Ok(content)
    }

    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)
    }

    pub fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_all(s.as_bytes())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Memory-mapped file for efficient I/O
pub struct MappedFile {
    map: memmap2::MmapMut,
    _file: std::fs::File,
}

impl MappedFile {
    pub fn create(path: &str, size: usize) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
            
        file.set_len(size as u64)?;
        
        let map = unsafe { memmap2::MmapMut::map_mut(&file)? };
        
        Ok(MappedFile {
            map,
            _file: file,
        })
    }

    pub fn open(path: &str) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
            
        let map = unsafe { memmap2::MmapMut::map_mut(&file)? };
        
        Ok(MappedFile {
            map,
            _file: file,
        })
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.map
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.map
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.map.flush()
    }
}

// Buffered I/O implementations
pub struct BufReader<R: Read> {
    inner: R,
    buffer: Vector<u8>,
    pos: usize,
    cap: usize,
}

impl<R: Read> BufReader<R> {
    pub fn new(inner: R) -> Self {
        BufReader {
            inner,
            buffer: Vector::with_capacity(8192),
            pos: 0,
            cap: 0,
        }
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        BufReader {
            inner,
            buffer: Vector::with_capacity(capacity),
            pos: 0,
            cap: 0,
        }
    }

    fn fill_buffer(&mut self) -> io::Result<()> {
        if self.pos >= self.cap {
            self.cap = self.inner.read(&mut self.buffer)?;
            self.pos = 0;
        }
        Ok(())
    }
}

impl<R: Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;
        
        while total_read < buf.len() {
            if self.pos >= self.cap {
                self.fill_buffer()?;
                if self.cap == 0 {
                    break;
                }
            }
            
            let available = self.cap - self.pos;
            let to_read = (buf.len() - total_read).min(available);
            
            buf[total_read..total_read + to_read]
                .copy_from_slice(&self.buffer[self.pos..self.pos + to_read]);
                
            self.pos += to_read;
            total_read += to_read;
        }
        
        Ok(total_read)
    }
}
