use std::env;
use std::path::PathBuf;

fn main() {
    // Set LLVM path for Windows
    if cfg!(windows) {
        println!("cargo:rustc-link-search=C:\\Program Files\\LLVM\\lib");
        env::set_var("LLVM_SYS_150_PREFIX", "C:\\Program Files\\LLVM");
    }

    // Configure LLVM
    let llvm_config = if cfg!(windows) {
        "C:\\Program Files\\LLVM\\bin\\llvm-config"
    } else {
        "llvm-config"
    };
    
    // Get LLVM flags and libraries
    let output = std::process::Command::new(llvm_config)
        .arg("--cxxflags")
        .output()
        .expect("Failed to execute llvm-config");
    let cxxflags = String::from_utf8_lossy(&output.stdout);
    
    let output = std::process::Command::new(llvm_config)
        .arg("--ldflags")
        .output()
        .expect("Failed to execute llvm-config");
    let ldflags = String::from_utf8_lossy(&output.stdout);

    // Print cargo configuration
    println!("cargo:rustc-flags={}", cxxflags);
    println!("cargo:rustc-flags={}", ldflags);
}
