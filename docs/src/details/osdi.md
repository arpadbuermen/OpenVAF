# OSDI - Simulator Interface

OpenVAF generates shared objects that can be loaded by circuit simulators at run-time.
To ensure compatibility with a wide variety of simulators, a **simulator independent** interface called OSDI (**O**pen **S**ource **D**evice **I**nterface) was developed.
This interface is very flexible and allows efficient integration with a wide variety of different simulators.
As a result it can support both modern harmonic balance solvers and traditional SPICE based engines.

An implementation that bridges the internal spice API and OSDI has been added to `ngspice`.

## OSDI Versions

OpenVAF-reloaded currently generates models with OSDI API version 0.4.

### OSDI 0.4 Changes

In OSDI 0.4 new members are added to the module descriptor data structure after the members defined in the OSDI 0.3 specification. The descriptor (if cast to the declaration given in the OSDI 0.3 header file) remains compatible with OSDI 0.3 and should work just like before.

Simulators using OSDI API 0.3 can be adapted to use version 0.4 by applying the following changes:
- allowing major.minor version >=0.4 beside 0.3,
- reading the `OSDI_DESCRIPTOR_SIZE` symbol of type `uint32` specifying the descriptor size,
- making sure the table of descriptors (pointed to by the `OSDI_DESCRIPTORS` symbol) is traversed in steps of size `OSDI_DESCRIPTOR_SIZE` instead of `sizeof(OsdiDescriptor)`, and
- casting each descriptor to the structure declared in the OSDI header file, version 0.3.

### Simulator Support

| Simulator | OSDI version supported | Comment |
|-----------|------------------------|---------|
| [Ngspice](https://ngspice.sourceforge.io/) 43 | 0.3 | |
| [Ngspice](https://ngspice.sourceforge.io/) >=44 | 0.3 & 0.4 | uses only 0.3 features |
| [SPICE OPUS](https://www.spiceopus.si/) 3.0 | 0.3 | |
| [VACASK](https://codeberg.org/arpadbuermen/VACASK) 0.1.2 | 0.3 | |
| [VACASK](https://codeberg.org/arpadbuermen/VACASK) >=0.2 | 0.4 | |

## What's New in OSDI 0.4

- OSDI descriptor size for traversing the OSDI descriptor table in simulators not supporting OSDI 0.4
- Support for reading param given flags of parameters in the instance and model data structures
- Support for writing nonzero resistive and reactive Jacobian contributions to an array of doubles
- List of model inputs (node pairs)
- Functions for loading Jacobians with offset (for harmonic balance analysis)
- Natures, disciplines, and the corresponding attributes exposed in OSDI API
- Natures of unknowns and residuals exposed in OSDI descriptor
