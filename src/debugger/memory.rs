use std::mem;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::Foundation::*;

pub struct MemoryInspector {
    process_handle: HANDLE,
}

#[derive(Debug)]
pub enum MemoryError {
    ReadError(String),
    WriteError(String),
    ProtectionError(String),
}

impl MemoryInspector {
    pub fn new(process_handle: HANDLE) -> Self {
        MemoryInspector { process_handle }
    }

    pub fn read_memory<T: Copy>(&self, address: usize) -> Result<T, MemoryError> {
        let mut buffer: T = unsafe { mem::zeroed() };
        let size = mem::size_of::<T>();

        unsafe {
            if ReadProcessMemory(
                self.process_handle,
                address as *const _,
                &mut buffer as *mut T as *mut _,
                size,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(MemoryError::ReadError(format!(
                    "Failed to read memory at address {:#x}",
                    address
                )));
            }
        }

        Ok(buffer)
    }

    pub fn write_memory<T: Copy>(&self, address: usize, value: T) -> Result<(), MemoryError> {
        let size = mem::size_of::<T>();

        unsafe {
            if WriteProcessMemory(
                self.process_handle,
                address as *mut _,
                &value as *const T as *const _,
                size,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(MemoryError::WriteError(format!(
                    "Failed to write memory at address {:#x}",
                    address
                )));
            }
        }

        Ok(())
    }

    pub fn read_string(&self, address: usize, max_length: usize) -> Result<String, MemoryError> {
        let mut buffer = vec![0u8; max_length];

        unsafe {
            if ReadProcessMemory(
                self.process_handle,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                max_length,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(MemoryError::ReadError(format!(
                    "Failed to read string at address {:#x}",
                    address
                )));
            }
        }

        // Find null terminator
        let length = buffer.iter().position(|&b| b == 0).unwrap_or(max_length);
        String::from_utf8_lossy(&buffer[..length]).to_string().parse().map_err(|e| {
            MemoryError::ReadError(format!("Invalid UTF-8 string at address {:#x}: {}", address, e))
        })
    }

    pub fn protect_memory(&self, address: usize, size: usize, protection: u32) -> Result<u32, MemoryError> {
        let mut old_protection = 0;

        unsafe {
            if VirtualProtectEx(
                self.process_handle,
                address as *mut _,
                size,
                protection,
                &mut old_protection,
            ) == 0
            {
                return Err(MemoryError::ProtectionError(format!(
                    "Failed to change memory protection at address {:#x}",
                    address
                )));
            }
        }

        Ok(old_protection)
    }

    pub fn scan_memory<T: Copy + PartialEq>(
        &self,
        start_address: usize,
        end_address: usize,
        value: T,
    ) -> Vec<usize> {
        let mut matches = Vec::new();
        let size = mem::size_of::<T>();
        let mut current_address = start_address;

        while current_address + size <= end_address {
            if let Ok(read_value) = self.read_memory::<T>(current_address) {
                if read_value == value {
                    matches.push(current_address);
                }
            }
            current_address += size;
        }

        matches
    }

    pub fn dump_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, MemoryError> {
        let mut buffer = vec![0u8; size];

        unsafe {
            if ReadProcessMemory(
                self.process_handle,
                address as *const _,
                buffer.as_mut_ptr() as *mut _,
                size,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(MemoryError::ReadError(format!(
                    "Failed to dump memory at address {:#x}",
                    address
                )));
            }
        }

        Ok(buffer)
    }

    pub fn format_memory_dump(&self, address: usize, data: &[u8]) -> String {
        let mut output = String::new();
        let bytes_per_line = 16;

        for (i, chunk) in data.chunks(bytes_per_line).enumerate() {
            // Address
            output.push_str(&format!("{:08x}  ", address + i * bytes_per_line));

            // Hex values
            for (j, &byte) in chunk.iter().enumerate() {
                output.push_str(&format!("{:02x}", byte));
                if j % 2 == 1 {
                    output.push(' ');
                }
            }

            // Padding for incomplete lines
            if chunk.len() < bytes_per_line {
                let padding = (bytes_per_line - chunk.len()) * 2 + (bytes_per_line - chunk.len() + 1) / 2;
                output.push_str(&" ".repeat(padding));
            }

            // ASCII representation
            output.push_str(" |");
            for &byte in chunk {
                if byte.is_ascii_graphic() {
                    output.push(byte as char);
                } else {
                    output.push('.');
                }
            }
            output.push_str("|\n");
        }

        output
    }
}
