// build.rs for openvaf-driver

fn main() {
    // Add rpath for LLVM on macOS
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("llvm-config")
            .arg("--libdir")
            .output()
        {
            if output.status.success() {
                let libdir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir);
            }
        }
    }
}
