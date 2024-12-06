use std::collections::HashMap;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::Foundation::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WatchType {
    Read,
    Write,
    ReadWrite,
}

pub struct Watchpoint {
    address: usize,
    size: usize,
    watch_type: WatchType,
    original_protection: u32,
    handler: Box<dyn Fn(usize, &[u8]) + Send + 'static>,
}

pub struct WatchpointManager {
    process_handle: HANDLE,
    watchpoints: HashMap<usize, Watchpoint>,
}

impl WatchpointManager {
    pub fn new(process_handle: HANDLE) -> Self {
        WatchpointManager {
            process_handle,
            watchpoints: HashMap::new(),
        }
    }

    pub fn add_watchpoint<F>(
        &mut self,
        address: usize,
        size: usize,
        watch_type: WatchType,
        handler: F,
    ) -> Result<(), String>
    where
        F: Fn(usize, &[u8]) + Send + 'static,
    {
        let mut protection = PAGE_NOACCESS;
        match watch_type {
            WatchType::Read => protection = PAGE_READONLY,
            WatchType::Write => protection = PAGE_READWRITE,
            WatchType::ReadWrite => protection = PAGE_READWRITE,
        }

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
                return Err("Failed to set memory protection".to_string());
            }
        }

        self.watchpoints.insert(
            address,
            Watchpoint {
                address,
                size,
                watch_type,
                original_protection: old_protection,
                handler: Box::new(handler),
            },
        );

        Ok(())
    }

    pub fn remove_watchpoint(&mut self, address: usize) -> Result<(), String> {
        if let Some(watchpoint) = self.watchpoints.remove(&address) {
            unsafe {
                let mut old_protection = 0;
                if VirtualProtectEx(
                    self.process_handle,
                    address as *mut _,
                    watchpoint.size,
                    watchpoint.original_protection,
                    &mut old_protection,
                ) == 0
                {
                    return Err("Failed to restore memory protection".to_string());
                }
            }
        }
        Ok(())
    }

    pub fn handle_exception(
        &self,
        exception_address: usize,
        exception_code: u32,
    ) -> Option<&Watchpoint> {
        // Find the watchpoint that contains the exception address
        self.watchpoints.values().find(|wp| {
            exception_address >= wp.address && exception_address < wp.address + wp.size
        })
    }

    pub fn notify_watchpoint(&self, watchpoint: &Watchpoint) -> Result<(), String> {
        let mut buffer = vec![0u8; watchpoint.size];
        unsafe {
            if ReadProcessMemory(
                self.process_handle,
                watchpoint.address as *const _,
                buffer.as_mut_ptr() as *mut _,
                watchpoint.size,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err("Failed to read memory at watchpoint".to_string());
            }
        }

        (watchpoint.handler)(watchpoint.address, &buffer);
        Ok(())
    }
}
