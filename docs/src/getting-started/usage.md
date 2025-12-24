# Verilog-A Compilation

Once OpenVAF is installed, compilation of Verilog-A models to an OSDI library suitable for circuit simulation is easy.
Simply run

```bash
openvaf-r <file>.va
```

in a terminal and the compilation should complete quickly.
OpenVAF offers options which are displayed when executing `openvaf-r --help`, but these won't usually be required.

If there are no errors in the Verilog-A source, a file called `<file>.osdi` will be generated that can be used by
circuit simulators that implement the [OSDI interface](../details/osdi.md), such as Ngspice.


# Ngspice Integration

## Loading OSDI Files

Once you have a compiled an OSDI file as described above, it can be loaded by Ngspice using a simple Ngspice simulator command.

```bash
osdi <path>.osdi
```

To load a model in a netlist, one must add `pre` to the  `osdi` command for ensuring that the model is loaded before the netlist is resolved:

```bash
.control
pre_osdi <file>.osdi
*other control commands
.endc
```

* The path to the file can be an absolute path like `/home/folder/example.osdi`.
* The path can be a relative path like `folder/example.osdi`, it is then resolved in the netlist directory.
* You can also omit the path and just write `example.osdi` if the file is located in the `lib/ngspice` directory. To activate this feature you must comment out `unset osdi_enabled` in `share/ngspice/scripts/spinit` . This method is recommended if you want to make this model permanently available to Ngspice.


## Using Verilog-A Modules

Once an OSDI file has been loaded, Verilog-A modules can be initiated in a netlist as shown below.
Note that the `N` prefix of the device name is important for ensuring correct behavior.

```bash
.model <model name> <Verilog-A module name> <model parameters>*

N<instance name> <nodes>* <model name> <instance parameters>*
```

A minimal example netlist that creates a single instance of the HICUM/L2 model is shown below.
Further examples can be found in the [Examples](./examples.md) section.

```bash
OSDI Example

.model hicum_model hicuml2va c10=1e-30

N1 C B E S hicum_model

.control
pre_osdi hicumL2V3p0p0.osdi
.endc
.end
```
