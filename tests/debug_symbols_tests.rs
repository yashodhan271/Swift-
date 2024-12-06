#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::debugger::symbols::{DebugSymbols, SourceLocation};

    #[test]
    fn test_load_debug_symbols() {
        let test_exe = PathBuf::from(env!("CARGO_BIN_EXE_swiftpp"));
        let symbols = DebugSymbols::new(&test_exe).expect("Failed to load debug symbols");
        
        // Verify we can load symbols
        assert!(!symbols.get_symbol_at_address(0).is_none(), "No symbols found");
    }

    #[test]
    fn test_find_symbol_by_address() {
        let test_exe = PathBuf::from(env!("CARGO_BIN_EXE_swiftpp"));
        let symbols = DebugSymbols::new(&test_exe).expect("Failed to load debug symbols");
        
        // Get first symbol
        let first_symbol = symbols.get_symbol_at_address(0)
            .expect("No symbols found");
        
        // Try to find symbol containing an address within its range
        let found_symbol = symbols.find_symbol_containing_address(first_symbol.address + 1)
            .expect("Failed to find symbol by address");
        
        assert_eq!(first_symbol.name, found_symbol.name);
    }

    #[test]
    fn test_source_location() {
        let test_exe = PathBuf::from(env!("CARGO_BIN_EXE_swiftpp"));
        let mut symbols = DebugSymbols::new(&test_exe).expect("Failed to load debug symbols");
        
        // Find a symbol with source location
        let symbol = symbols.get_symbol_at_address(0)
            .expect("No symbols found");
        
        if let Some(location) = &symbol.location {
            symbols.load_source_file(&location.file)
                .expect("Failed to load source file");
            
            let source_line = symbols.get_source_line(location)
                .expect("Failed to get source line");
            
            assert!(!source_line.is_empty(), "Empty source line");
        }
    }

    #[test]
    fn test_local_variables() {
        let test_exe = PathBuf::from(env!("CARGO_BIN_EXE_swiftpp"));
        let symbols = DebugSymbols::new(&test_exe).expect("Failed to load debug symbols");
        
        // Find a symbol with local variables
        let symbol = symbols.get_symbol_at_address(0)
            .expect("No symbols found");
        
        let variables = symbols.get_local_variables(symbol.address);
        assert!(variables.is_some(), "No local variables found");
    }
}
