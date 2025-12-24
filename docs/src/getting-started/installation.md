# Installation

OpenVAF is available as a pre-compiled standalone executable for Windows and Linux.
Compiling OpenVAF yourself is possible but not recommended as setting up the dependencies is difficult at the moment.

## System Requirements

OpenVAF supports:

* All Unix systems released from RHEL 7 on.
* Windows systems from Windows 10 on.
* macOS (with Homebrew for LLVM dependencies)

OpenVAF requires that a **linker is installed**:
* On Linux the `ld` linker must be available in the path (typically already installed together with gcc).
* On Windows the MSVC build tools must be downloaded and installed [from the microsoft website](https://visualstudio.microsoft.com/downloads/?q=build+tools#build-tools-for-visual-studio-2022).
* On macOS, Xcode command line tools provide the necessary linker.

## Pre-compiled Executable

Pre-compiled OpenVAF executables for supported platforms are available from the releases page.
After download, place the **openvaf-r** executable in your PATH, so that it can be called from everywhere.

Try to run

```bash
openvaf-r --help
```

in a terminal and see if the executable is found.

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

## Compilation

Users can compile OpenVAF themselves.
This is not recommended for users that do not want to actively take part in the development of OpenVAF
since the build process is quite involved because of its **LLVM dependency**.

The compilation is detailed in the **README.md** of the [OpenVAF repository](https://github.com/arpadbuermen/OpenVAF).
