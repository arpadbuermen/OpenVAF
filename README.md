<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_light-r.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_dark-r.svg">
  <img alt="OpenVAF" src="logo_dark-r.svg">
</picture>

<br>    
<br>
<br>

## About this repository

This is a fork of [Pascal Kuthe's OpenVAF repository](https://github.com/pascalkuthe/OpenVAF) by Arpad
Buermen. Since the OSDI interface used by the compiled models has been extended the project was renamed to **OpenVAF-reloaded**. Several bugs have been fixed that prevented OpenVAF from compiling SPICE-like Verilog-A models that could replace builtin SPICE3 models. The last version that uses OSDI interface v0.3 is tagged with `osdi_0.3`. After that the OSDI version has been bumped to 0.4 and the generated binary is now called `openvaf-r` (as in OpenVAF-reloaded). 

## OSDI 0.4

In OSDI 0.4 new members are going to be added to the module descriptor data structure. The descriptor (if cast to the declaration given in the OSDI 0.3 header file) remains compatible with OSDI 0.3 and should work just like before. Ngspice no longer works with openvaf-r generated models. It could, however, support the new OSDI format by 
- allowing major.minor version 0.4 besides 0.3, 
- reading the `OSDI_DESCRIPTOR_SIZE` symbol of type `uint32` specifying the descriptor size, 
- making sure the table of descriptors (pointed to by the `OSDI_DESCRIPTORS` symbol) is traversed in steps of size `OSDI_DESCRIPTOR_SIZE`, and
- casting each descriptor to the structure declared in the OSDI header file, version 0.3. 

What is on the TODO list?
- Write a patch for Ngspice that will allow it to use OSDI 0.4 models. 
- Clean up the repository and keep only OpenVAF. 
- Update OSDI documentation so that it includes OSDI 0.4 features. 

Currently I do not have time for that. :) Feel free to do that yourself and send me a pull request. 

What is new in OSDI 0.4? 
- Support for reading param given flags of parameters in the instance and model data structures. This is pretty much self-explanatory. Look at the OSDI 0.4 header file. 
 

## Setting up the dependencies under Debian Bookworm

Get [LLVM 15 built by Pascal](https://openva.fra1.cdn.digitaloceanspaces.com/llvm-15.0.7-x86_64-unknown-linux-gnu-FULL.tar.zst) 
(do not use the Debian-supplied version). You can also build your own LLVM and Clang 15.0.7 from 
[sources](https://github.com/llvm/llvm-project/releases/tag/llvmorg-15.0.7).  

Unpack Pascal's binaries in `/opt` as root (creates directory `/opt/LLVM`). You will need zstd for that. 
```
cd /opt
zstd -d -c --long=31 <path/to/archive.tar.zst> | tar -xf -
```
Install Rust as ordinary user (files will go to `~/.cargo` and `~/.rustup`). 
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
During installation select "Customize installation" and set profile to "complete". 

Set LLVM_CONFIG, add LLVM to PATH, and set up the working environment for Rust.
Add these lines at the end of `.bashrc`
```
. "$HOME/.cargo/env"
export LLVM_CONFIG=/opt/LLVM/bin/llvm-config
export PATH=/opt/LLVM/bin:$PATH
```

Restart shell. You're good to go. 


## Setting up the dependencies under Windows

Download [rustup](https://win.rustup.rs), run it to install Rust. 
During installation select "Customize installation" and set profile to "complete". 

Install Visual Studio 2019 Community Edition (tested with version 16.11.33) 
Make sure you install CMake Tools that come with VS2019 (also installs Ninja). 

Build LLVM and Clang, download [LLVM 15.0.7](https://github.com/llvm/llvm-project/releases/tag/llvmorg-15.0.7) sources (get the .zip file)

Unpack the sources. This creates directory `llvm-project-llvmorg-15.0.7`. Create a directory named `build`. 

Start Visual Studio x64 native command prompt. 
Run CMake, use Ninja as build system. Do not use default (nmake) because for me it always built the Debug version, even when I specified Release. 
Replace `e:\llvm` with the path where you want yout LLVM and Clang binaries and libraries to be installed. 
```
cmake -G Ninja -S llvm-project-llvmorg-15.0.7\llvm -B build -DCMAKE_INSTALL_PREFIX=e:\LLVM -DCMAKE_BUILD_TYPE=Release -DLLVM_TARGETS_TO_BUILD="X86;ARM;AArch64" -DLLVM_ENABLE_PROJECTS="llvm;clang"
```
Run Ninja (build and install)
```
ninja -C build
ninja -C build install 
```
Now you have your own LLVM and Clang. Hope it did not take too many Snickers :). 

The LLVM and Clang version [built by Pascal](https://openva.fra1.cdn.digitaloceanspaces.com/llvm-15.0.7-x86_64-pc-windows-msvc-FULL.tar.zst) did not work for me (the openvaf binary failed to link due to undefined symbols). 

Add LLVM to the PATH (in the above example that would be `e:\llvm\bin`). 
Set the `LLVM_CONFIG` environmental variable if you have multiple LLVM installations
(for the above example that would be `e:\llvm\bin\llvm-config.exe`). 

Restart command prompt. Now you are good to go. 


## Building

To build the release version (`target/release/openvaf-r`), type
```
cargo build --release --bin openvaf-r
```

To build the debug version (`target/debug/openvaf-r`), type
```
cargo build --bin openvaf-r
```


## Acknowledgement

Kudos to Pascal Kuthe for the great work he did. 


## Copyright

This work is free software and licensed under the GPL-3.0 license.
It contains code that is derived from [rustc](https://github.com/rust-lang/rust/) and [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer). These projects are both licensed under the MIT license. As required a copy of the license and disclaimer can be found in `copyright/LICENSE_MIT`.

Many models in integration tests folder are not licensed under a GPL compatible license. All of those models contain explicit license information. They do not endup in the openvaf binary in any way and therefore do not affect the license of the entire project. Integration tests without explicit model information (either in the model files or in a dedicated LICENSE file) fall under GPLv3.0 like the rest of the repo.
