# Compact Models

This pages provides an in-complete overview of Verilog-A compact models that are available online.
All of these models (and more) are automatically tested during OpenVAF development to ensure they work correctly.
Do not hesitate to open an issue if we forgot your favorite model!

## CMC Models

The [Compact Model Coalition (CMC)](https://si2.org/cmc/) is a working collaborative group focused on the standardization of SPICE (Simulation Program with Integration Circuit Emphasis) device models.
CMC models are available in most commercial circuit simulators and are widely used in industry and academia.

### BSIM (FET)

[The BSIM group](http://bsim.berkeley.edu/) develops and releases **several FET models**:

* [BSIM4](https://github.com/cogenda/VA-BSIM48) is an older MOSFET model that only has a unofficial Verilog-A file.
* [BSIM-CMG](https://bsim.berkeley.edu/models/bsimcmg/) for common multi-gate MOSFETs like FinFETs.
* [BSIM-SOI](https://bsim.berkeley.edu/models/bsimsoi/) for SOI (Silicon-On-Insulator) MOSFETs.
* [BSIM-IMG](https://bsim.berkeley.edu/models/bsimimg/) for independent double-gate structures like Ultra-Thin Body and BOX SOI transistors (UTBB).
* [BSIM-BULK](https://bsim.berkeley.edu/models/bsimbulk/) for state-of-the-art bulk FETs.

### PSP Model (FET)

PSP is a Compact Model for bulk MOSFETs and is available [here](https://www.cea.fr/cea-tech/leti/pspsupport/CurrentRelease).

### JUNCAP Model (Diode)

[The juncap diode model](https://www.cea.fr/cea-tech/leti/pspsupport/Pages/Welcome.aspx) is developed by NXP Research and CEA-Leti.

### HiSIM (FET)

[HiSIM](https://www.hisim.hiroshima-u.ac.jp/cgi/HiSIM2/public_release.cgi) (Hiroshima-university STARC IGFET Model) is the first complete surface-potential-based MOS-
FET model for circuit simulation based on the drift-diffusion approximation.

### L-UTSOI (SOI-FET)

The [L-UTSOI compact model](https://www.cea.fr/cea-tech/leti/l-utsoisupport) is dedicated to FDSOI technologies, and is the new name of Leti-UTSOI,
 a high maturity model in development since 2007 and used in industrial environments for nearly 8 years.

### HICUM (Bipolar Transistors)

The [HICUM/L2 and HICUM/L0 models](https://www.iee.et.tu-dresden.de/iee/eb/hic_new/hic_intro.html) are developed by Prof. Michael Schröter.
HICUM stands for HIgh CUrrent Model and is a physics-based geometry-scalable compact model for homo- and heterojunction bipolar transistors.
It targets the design of circuits using Si, SiGe or III-V based processes and is particularly accurate at high-frequencies and high-current densities.

### ASM-HEMT (GAN HEMT)

The [ASM-HEMT model](https://iitk.ac.in/asm/) is a robust surface-potential based compact model for Gallium Nitride (GaN) High Electron Mobility Transistors.
It was developed as a joint effort between Indian Institute of Technology Kanpur and Macquarie University.



## Non CMC Models

There are models that are released without CMC involvement. Some of these models are listed below.

### EKV (FET)

[The EKV2.6 MOSFET compact model](https://github.com/ekv26/model) has had a considerable impact on the academic and industrial community of analog integrated circuit design, since its inception in 1996. The model is available as a free open-source software (FOSS) tool coded in Verilog-A. The present depository provides a short review of foundations of the model and shows its capabilities via characterization and modeling based on a test chip in 180 nm CMOS fabricated via Europractice.
