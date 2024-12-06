# Swift++ Installation Guide

This guide provides detailed instructions for installing Swift++ and its dependencies on various operating systems.

## System Requirements

- 64-bit processor
- 4GB RAM minimum (8GB recommended)
- 2GB free disk space
- Operating System:
  - Windows 10/11
  - Ubuntu 20.04 or later
  - macOS 11.0 or later

## Dependencies

### Required Dependencies
1. LLVM 15.0.0 or later
2. Rust 1.56.0 or later
3. CMake 3.10 or later
4. C++17 compatible compiler

### Optional Dependencies
1. Ninja build system (recommended)
2. Python 3.7 or later (for development tools)

## Installation Steps

### Windows

1. Install Visual Studio 2019 or later with C++ support:
   - Download from [Visual Studio Downloads](https://visualstudio.microsoft.com/downloads/)
   - Include "Desktop development with C++"

2. Install Rust:
   ```powershell
   winget install Rustlang.Rust
   ```

3. Install LLVM:
   ```powershell
   winget install LLVM.LLVM
   ```

4. Add LLVM to PATH:
   ```powershell
   $env:Path += ";C:\Program Files\LLVM\bin"
   ```

5. Install CMake:
   ```powershell
   winget install Kitware.CMake
   ```

### Ubuntu/Debian

1. Install system dependencies:
   ```bash
   sudo apt update
   sudo apt install build-essential cmake ninja-build python3
   ```

2. Install LLVM:
   ```bash
   wget https://apt.llvm.org/llvm.sh
   chmod +x llvm.sh
   sudo ./llvm.sh 15
   ```

3. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

4. Configure environment:
   ```bash
   source $HOME/.cargo/env
   ```

### macOS

1. Install Xcode Command Line Tools:
   ```bash
   xcode-select --install
   ```

2. Install Homebrew:
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

3. Install dependencies:
   ```bash
   brew install llvm@15 cmake ninja rust
   ```

4. Configure LLVM path:
   ```bash
   echo 'export PATH="/usr/local/opt/llvm@15/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

## Building Swift++

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/swiftpp.git
   cd swiftpp
   ```

2. Build the compiler:
   ```bash
   cargo build --release
   ```

3. Add to PATH:
   ```bash
   # Linux/macOS
   echo 'export PATH="$PATH:$PWD/target/release"' >> ~/.bashrc
   source ~/.bashrc

   # Windows PowerShell
   $env:Path += ";$PWD\target\release"
   ```

## Verification

1. Verify installation:
   ```bash
   swiftpp --version
   ```

2. Run test suite:
   ```bash
   cargo test
   ```

3. Try compiling an example:
   ```bash
   swiftpp examples/hello.spp -o hello
   ./hello
   ```

## Troubleshooting

### Common Issues

1. LLVM not found:
   - Ensure LLVM is properly installed
   - Check PATH environment variable
   - Verify llvm-config is accessible

2. Compilation errors:
   - Update to latest Rust toolchain
   - Verify C++ compiler installation
   - Check CMake version

3. Linker errors:
   - Install required development libraries
   - Update system PATH
   - Check library versions

### Getting Help

- File issues on GitHub
- Join our Discord community
- Check documentation
- Contact support team

## Uninstallation

To remove Swift++:

1. Remove binary:
   ```bash
   # Linux/macOS
   rm -rf ~/.cargo/bin/swiftpp

   # Windows
   Remove-Item "$env:USERPROFILE\.cargo\bin\swiftpp.exe"
   ```

2. Remove repository:
   ```bash
   rm -rf /path/to/swiftpp
   ```

3. Optional: Remove dependencies
   ```bash
   # Ubuntu/Debian
   sudo apt remove llvm-15-dev

   # macOS
   brew uninstall llvm@15

   # Windows
   winget uninstall LLVM.LLVM
   ```
