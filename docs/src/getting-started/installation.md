# Installation

OpenVAF-R is available as a pre-compiled standalone executable for Windows, Linux, and macOS.
Compiling OpenVAF yourself is possible but not recommended as setting up the dependencies is difficult at the moment.

## System Requirements

OpenVAF supports:

* Linux: RHEL 7+ compatible distributions
* Windows: Windows 10 and later
* macOS: macOS 13+ (Ventura or later), Apple Silicon and Intel

OpenVAF requires that a **linker is installed**:
* On Linux the `ld` linker must be available in the path (typically already installed together with gcc).
* On Windows the MSVC build tools must be downloaded and installed [from the microsoft website](https://visualstudio.microsoft.com/downloads/?q=build+tools#build-tools-for-visual-studio-2022).
* On macOS, Xcode command line tools provide the necessary linker (`xcode-select --install`).

## macOS (Homebrew) - Recommended

The easiest way to install OpenVAF-R on macOS is via Homebrew:

```bash
brew tap robtaylor/openvaf
brew install openvaf-r
```

Or install directly:

```bash
brew install robtaylor/openvaf/openvaf-r
```

To install the latest development version (builds from source):

```bash
brew install --HEAD robtaylor/openvaf/openvaf-r
```

To update:

```bash
brew update && brew upgrade openvaf-r
```

## Pre-compiled Executable

Pre-compiled OpenVAF executables for supported platforms are available from the [releases page](https://github.com/robtaylor/OpenVAF/releases).

After download:
1. Extract the archive
2. Place the **openvaf-r** executable in your PATH

Verify the installation:

```bash
openvaf-r --version
openvaf-r --help
```

## Ngspice Compatibility

> **Note:** Support for OSDI/OpenVAF is available starting with the Ngspice-39 release. Earlier versions of Ngspice cannot be used.
>
> On Linux you can either install Ngspice with your package manager (if ngspice-39 is already available). Alternatively you can build Ngspice from source as follows:
>
> ```bash
> git clone git://git.code.sf.net/p/ngspice/ngspice
> cd ngspice
> sudo ./compile_linux.sh
> ```
>
> Note that Ngspice has dependencies that must be installed on the system.
> Consult the [Ngspice website](https://ngspice.sourceforge.io/) for details regarding Ngspice installation.
>
> For Windows a precompiled version is [available on the Ngspice website](https://ngspice.sourceforge.io/download.html).

## Building from Source

Users can compile OpenVAF themselves.
This is not recommended for users that do not want to actively take part in the development of OpenVAF
since the build process is quite involved because of its **LLVM dependency**.

### Requirements

* Rust toolchain (stable)
* LLVM 18
* C compiler (gcc, clang, or MSVC)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/robtaylor/OpenVAF.git
cd OpenVAF

# Set LLVM path (example for Homebrew on macOS)
export LLVM_SYS_181_PREFIX=$(brew --prefix llvm@18)

# Build release binary
cargo build --release

# The binary is at target/release/openvaf-r
```

See the [README.md](https://github.com/robtaylor/OpenVAF) for more detailed build instructions.
