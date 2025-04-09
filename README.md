<picture>
  <source media="(prefers-color-scheme: dark)" srcset="logo_light-r.svg">
  <source media="(prefers-color-scheme: light)" srcset="logo_dark-r.svg">
  <img alt="OpenVAF" src="logo_dark-r.svg">
</picture>

<br>    
<br>
<br>

# OpenVAF-reloaded

OpenVAF is a Verilog-A compiler written by Pascal Kuthe. The compiler outputs a dynamic library whose functionality can be accessed via the OSDI API (version 0.3). The original compiler received no support since end of 2023. This fork of [the original repository](https://github.com/pascalkuthe/OpenVAF) was started by Árpád Bűrmen in early 2024. Since then several small bugs were fixed that prevented the use of OpenVAF for building SPICE3-equivalent device models. 

To add new functionality to OpenVAF the OSDI interface has been modified. Consequently the current version of OSDI API is 0.4. OSDI API 0.4 differs from version 0.3 in the module descriptor. It also exports `OSDI_DESCRIPTOR_SIZE` which can be used to traverse the array of descriptors without relying on the definition of the `OsdiDescriptor` structure (i.e. size of the structure in the OSDI header file used by the simulator). New members are added after the first part of the descriptor which still complies with the OSDI 0.3 specification. Simulators that support only OSDI 0.3 can still use models exposing the newer OSDI API by applying some minor changes. 

The last version of OpenVAF before the project was renamed to **OpenVAF-reloaded** and the binary was renamed to `openvaf-r` is tagged with `osdi_0.3`. Currently two branches are maintained. The `master` branch includes several extensions of the compiler and exposes the OSDI 0.4 API in the generated models. The models generated by the compiler in the `branches/osdi_0.3` branch expose the old OSDI 0.3 API. This branch does not include compiler extensions as they depend on OSDI API 0.4. Both branches include all the bugfixes. 


# OSDI 0.4

In OSDI 0.4 new members are added to the module descriptor data structure after the members defined in the OSDI 0.3 specification. The descriptor (if cast to the declaration given in the OSDI 0.3 header file) remains compatible with OSDI 0.3 and should work just like before. Simulators using OSDI API 0.3 can be adapted to use version 0.4 by applying the following changes 
- allowing major.minor version >=0.4 beside 0.3, 
- reading the `OSDI_DESCRIPTOR_SIZE` symbol of type `uint32` specifying the descriptor size, 
- making sure the table of descriptors (pointed to by the `OSDI_DESCRIPTORS` symbol) is traversed in steps of size `OSDI_DESCRIPTOR_SIZE` instead of `sizeof(OsdiDescriptor)`, and
- casting each descriptor to the structure declared in the OSDI header file, version 0.3. 

This is the current state of OSDI 0.4 support

|Simulator|OSDI version supported|Comment|
|---------------|------------|---------------------------------------------------------------|
|[Ngspice](https://ngspice.sourceforge.io/) 43            |0.3         |        |
|[Ngspice](https://ngspice.sourceforge.io/) 44            |0.3 & 0.4   |uses only 0.3 features        |
|[SPICE OPUS](https://www.spiceopus.si/) 3.0              |0.3         |                                                               |
|[VACASK](https://codeberg.org/arpadbuermen/VACASK) 0.1.2 |0.3         |                                                               |
|[VACASK](https://codeberg.org/arpadbuermen/VACASK) 0.2   |0.4         |                                                               |

If you know of any other simulator supporting OSDI models generated by OpenVAF, let me know. 

Some internals of the OpenVAF compiler are documented in the [internals.md](internals.md) file. 

## What is new in OSDI 0.4 and OpenVAF in general? 

- OSDI descriptor size for traversing the OSDI descriptor table in simulators not supporting OSDI 0.4 
- Support for reading param given flags of parameters in the instance and model data structures. This is pretty much self-explanatory. Look at the [OSDI 0.4 header file](openvaf/osdi/header/osdi_0_4.h). This one takes care of issue #76 in the original repository. 
- Support for writing nonzero resistive and reactive Jacobian contributions to an array of doubles. 
- List of model inputs (node pairs). 
- Functions for loading Jacobians with offset (for harmonic balance analysis). 
- --dump_unopt_mir, --dump-mir, and --dump-ir options for dumpring the (unoptimized) MIR and LLVM IR. 
- Support for $fatal, $finish, and $stop. 


# What about binaries? 

Yes, binaries for 64-bit Linux and Windows are available [here](https://fides.fe.uni-lj.si/openvaf/download). The naming scheme of the binaries is 

```
openvaf-reloaded-<version>-<platform>
```

The version name is generated with `git --describe`. The OpenVAF-reloaded that produces models with the OSDI API 0.3 is version `osdi_0.3`. All newer versions (`osdi_0.4`) produce models with OSDI API 0.4. 

If the binary is named `openvaf` it comes from the `branches/osdi_0.3` branch and produces models with the OSDI 0.3 API. If the binary is named `openvaf-r` it comes from the `master` branch and produces models with the OSDI 0.4 API. 


# Building OpenVAF-reloaded

## Setting up the dependencies under Debian Bookworm

Everything was tested under Debian 13. Under Debian 12 a part of the OpenVAF suite fails to build (VerilogAE). 

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

### But I want to build my own LLVM...

Sure, no problem. Download the sources and unpack them. Then create a build directory and decide where you want to install LLVM. Type 
```
cmake -S <path to souces> -B <path to build dir> -DCMAKE_INSTALL_PREFIX=<install directory> -DCMAKE_BUILD_TYPE=Release -DLLVM_TARGETS_TO_BUILD="X86;ARM;AArch64" -DLLVM_ENABLE_PROJECTS="llvm;clang;lld"
```

Enter the build directory and type
```
make -j <number of processors to use>
make install
```

Set up the environment by adding the following to `.bashrc`
```
export LLVM_CONFIG=<install directory>/bin/llvm-config
export PATH=<install directory>/bin:$PATH
```

## Setting up the dependencies under Windows

Download [rustup](https://win.rustup.rs), run it to install Rust. 
During installation select "Customize installation" and set profile to "complete". 

Install Visual Studio 2019 Community Edition (tested with version 16.11.33) 
Make sure you install CMake Tools that come with VS2019 (also installs Ninja). 

Build LLVM and Clang, download [LLVM 15.0.7](https://github.com/llvm/llvm-project/releases/tag/llvmorg-15.0.7) sources (get the .zip file)

Unpack the sources. This creates directory `llvm-project-llvmorg-15.0.7`. Create a directory named `build`. 

Start Visual Studio x64 native command prompt. 
Run CMake, use Ninja as build system. Do not use default (nmake) because for me it always built the Debug version, even when I specified Release. 
Replace `e:\llvm` with the path where you want your LLVM and Clang binaries and libraries to be installed. 
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

# Debugging OpenVAF-reloaded in Visual Studio Code 

You will need two extensions: CodeLLDB (under Linux) / Microsoft C++ (under Windows) and rust-analyzer. In the `.vscode` directory there are two files: `launch-openvaf-r.json` (for working with the master branch) and `launch-openvaf.json` (for working with the branches/osdi_0.3 branch). Copy the one that matches your branch to `launch.json`. There are two debug setups available in that file (Linux and Windows). Set your breakpoints and run the program. If there are any changes since the last build they will be applied upon which the program will be started and then stop at the first breakpoint. 

The debug configuration disables rayon running the .osdi file build process in parallel so that debugging the last step of compilation is somewhat easier. 


# Running tests with cargo

Pascal has set up a test suite for OpenVAF. To run the tests on the debug version of the binary type

    cargo test

To run the tests on the release version type

    cargo test --release

By default only fast tests are run. To run all tests set the `RUN_SLOW_TEST` variable to 1, e.g. 

    RUN_SLOW_TESTS=1 cargo test 

Your changes may fail some tests although they are correct. Consider the case you changed the MIR generator. The expected test results assume MIR is generated the way Pascal did it. If you are sure your changes are correct you can update the expected values (stored in `openvaf/test_data` as files ending with .snap). To do this set the `UPDATE_EXPECT` variable 1, e.g. 

    UPDATE_EXPECT=1 cargo test

Unfortunately not all expected results are in .snap files. Some are hard-coded in the test sources, e.g. see `openvaf/mir_autodiff/src/builder/tests.rs`. You will have to update these expected values manually. 


# Acknowledgement

Kudos to Pascal Kuthe for the great work he did. 

Geoffrey Coram and Dietmar Warning are authors of several bugfixes included in OpenVAF-reloaded. 


# Copyright

This work is free software and licensed under the GPL-3.0 license.
It contains code that is derived from [rustc](https://github.com/rust-lang/rust/) and [rust-analyzer](https://github.com/rust-analyzer/rust-analyzer). These projects are both licensed under the MIT license. As required a copy of the license and disclaimer can be found in `copyright/LICENSE_MIT`.

Many models in integration tests folder are not licensed under a GPL compatible license. All of those models contain explicit license information. They do not end up in the openvaf binary in any way and therefore do not affect the license of the entire project. Integration tests without explicit model information (either in the model files or in a dedicated LICENSE file) fall under GPLv3.0 like the rest of the repo.
