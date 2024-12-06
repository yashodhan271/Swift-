use std::collections::HashMap;
use std::path::{Path, PathBuf};
use gimli::{self, Dwarf, EndianSlice, RunTimeEndian};
use object::{Object, ObjectSection};
use memmap2::Mmap;
use std::fs::File;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug)]
pub struct SymbolInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub location: Option<SourceLocation>,
}

#[derive(Debug)]
pub struct LocalVariable {
    pub name: String,
    pub type_name: String,
    pub location_expression: Vec<u8>,
}

pub struct DebugSymbols {
    dwarf: Dwarf<EndianSlice<RunTimeEndian>>,
    symbols: HashMap<u64, SymbolInfo>,
    variables: HashMap<u64, Vec<LocalVariable>>,
    source_files: HashMap<PathBuf, String>,
}

impl DebugSymbols {
    pub fn new(executable_path: &Path) -> Result<Self, String> {
        let file = File::open(executable_path)
            .map_err(|e| format!("Failed to open executable: {}", e))?;
        
        let map = unsafe {
            Mmap::map(&file)
                .map_err(|e| format!("Failed to map executable: {}", e))?
        };

        let object = object::File::parse(&*map)
            .map_err(|e| format!("Failed to parse executable: {}", e))?;

        // Load DWARF sections
        let endian = if object.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };

        let load_section = |id: gimli::SectionId| -> Result<EndianSlice<RunTimeEndian>, String> {
            let data = object
                .section_by_name(id.name())
                .and_then(|section| section.uncompressed_data().ok())
                .unwrap_or(std::borrow::Cow::Borrowed(&[]));
            Ok(EndianSlice::new(data.as_ref(), endian))
        };

        let dwarf = Dwarf::load(&load_section)
            .map_err(|e| format!("Failed to load DWARF info: {}", e))?;

        let mut symbols = HashMap::new();
        let mut variables = HashMap::new();
        let mut source_files = HashMap::new();

        // Parse debug information
        let mut unit_headers = dwarf.units();
        while let Ok(Some(header)) = unit_headers.next() {
            let unit = dwarf.unit(header)
                .map_err(|e| format!("Failed to parse unit: {}", e))?;

            let mut entries = unit.entries();
            while let Ok(Some((delta_depth, entry))) = entries.next_dfs() {
                if delta_depth >= 0 {
                    if let Ok(Some(name)) = entry.attr_value(gimli::DW_AT_name) {
                        if let Ok(name) = dwarf.attr_string(&unit, name) {
                            if let Ok(name) = name.to_string() {
                                if let Ok(Some(low_pc)) = entry.attr_value(gimli::DW_AT_low_pc) {
                                    if let gimli::AttributeValue::Addr(address) = low_pc {
                                        let size = entry
                                            .attr_value(gimli::DW_AT_high_pc)
                                            .ok()
                                            .and_then(|v| v)
                                            .and_then(|v| match v {
                                                gimli::AttributeValue::Udata(s) => Some(s),
                                                gimli::AttributeValue::Addr(h) => Some(h - address),
                                                _ => None,
                                            })
                                            .unwrap_or(0);

                                        let location = Self::get_source_location(&dwarf, &unit, entry);
                                        
                                        symbols.insert(address, SymbolInfo {
                                            name,
                                            address,
                                            size,
                                            location,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(DebugSymbols {
            dwarf,
            symbols,
            variables,
            source_files,
        })
    }

    fn get_source_location(
        dwarf: &Dwarf<EndianSlice<RunTimeEndian>>,
        unit: &gimli::Unit<EndianSlice<RunTimeEndian>>,
        entry: &gimli::DebuggingInformationEntry<EndianSlice<RunTimeEndian>>,
    ) -> Option<SourceLocation> {
        let file = entry.attr_value(gimli::DW_AT_decl_file).ok()??;
        let line = entry.attr_value(gimli::DW_AT_decl_line).ok()??;
        let column = entry.attr_value(gimli::DW_AT_decl_column).ok()?;

        let file_index = match file {
            gimli::AttributeValue::FileIndex(index) => index,
            _ => return None,
        };

        let line_program = match dwarf.line_program(unit) {
            Ok(Some(program)) => program,
            _ => return None,
        };

        let header = line_program.header();
        let file_entry = header.file(file_index)?;
        
        let mut file_path = PathBuf::new();
        if let Some(dir) = file_entry.directory(header) {
            if let Ok(dir_string) = dwarf.attr_string(unit, dir) {
                if let Ok(dir_str) = dir_string.to_string() {
                    file_path.push(dir_str);
                }
            }
        }

        if let Ok(file_string) = dwarf.attr_string(unit, file_entry.path_name()) {
            if let Ok(file_str) = file_string.to_string() {
                file_path.push(file_str);
            }
        }

        let line = match line {
            gimli::AttributeValue::Udata(l) => l as u32,
            _ => return None,
        };

        let column = match column {
            Some(gimli::AttributeValue::Udata(c)) => c as u32,
            _ => 0,
        };

        Some(SourceLocation {
            file: file_path,
            line,
            column,
        })
    }

    pub fn get_symbol_at_address(&self, address: u64) -> Option<&SymbolInfo> {
        self.symbols.get(&address)
    }

    pub fn find_symbol_containing_address(&self, address: u64) -> Option<&SymbolInfo> {
        self.symbols.values().find(|symbol| {
            address >= symbol.address && address < symbol.address + symbol.size
        })
    }

    pub fn get_source_line(&self, location: &SourceLocation) -> Option<&str> {
        self.source_files.get(&location.file).map(|content| {
            content.lines().nth((location.line - 1) as usize).unwrap_or("")
        })
    }

    pub fn get_local_variables(&self, frame_address: u64) -> Option<&Vec<LocalVariable>> {
        self.variables.get(&frame_address)
    }

    pub fn load_source_file(&mut self, path: &Path) -> Result<(), String> {
        if !self.source_files.contains_key(path) {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read source file: {}", e))?;
            self.source_files.insert(path.to_path_buf(), content);
        }
        Ok(())
    }

    pub fn evaluate_location_expression(
        &self,
        expression: &[u8],
        frame_base: u64,
        registers: &HashMap<u16, u64>,
    ) -> Result<u64, String> {
        // Implement DWARF expression evaluation
        // This is a simplified version - real implementation would need to handle
        // all DWARF expression operations
        Ok(0)
    }
}
