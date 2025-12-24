# FAQ

Answers to frequently asked questions.

### Why did you develop OpenVAF?

Coming from a modeling/software-engineering/academic background we noticed at the beginning of 2019 that
existing Verilog-A compilers had serious shortcomings.
For example slow compilation and simulation speed, questionable Verilog-A standard compliance and convergence issues.
Even worse, the ADMS tool used in the open-source circuit simulators is notoriously buggy and slow,
and unmaintained.

This problem became particularly pressing in recent years as CMC models moved to using newer Verilog-A features
that are not supported by ADMS.
This caused problems for both open source and commercial simulators using this tool.

### What are the main benefits of OpenVAF?

OpenVAF directly generates executable machine code and does not use an intermediate language such as C.
The compilation is very fast (up to 10 times faster than alternatives).
Furthermore, this approach allows implementation of better auto-differentiation algorithms
that can create highly optimized machine code that runs faster than code produced by traditional compilation approaches.

OpenVAF takes Verilog-A standard compliance seriously.
The approach described above allows OpenVAF to support many Verilog-A features without
sacrificing compilation or simulation performance.
As a result OpenVAF compiles a wide variety of standard compliant models
without modifications while offering better performance than other compilers.

OpenVAF also puts a great focus on ease of use.
No complicated setup is required to install the compiler.

### Will OpenVAF remain open-source?

Yes, OpenVAF and the OSDI specification will always remain open-source projects.
However, SemiMod offers potential commercial partners to build or integrate OpenVAF into proprietary tools.

### Why is additional parameter information like description, units and type (instance parameters) not available for some models?

Some older Verilog-A models use non-standard syntax for indicating parameter information
that can not be supported by OpenVAF.
Recently released compact models usually use the newer (correct) syntax.

The old syntax looks as follows:

```
paramter real example=1.0 (*type="instance"*);
```

The syntax is often abbreviated with a macro that disables the syntax outside ADMS:

```
paramter real example=1.0 `P(type="instance");
```

The Verilog-A standard (and therefore OpenVAF) supports the following syntax:

```
(*type="instance"*) paramter real example=1.0;
```


### I need help integrating OpenVAF into a simulator, can you help?

Please get in touch via the GitHub repository to determine the best path for cooperation.

### I need help extracting model parameters for a technology, can you help?

Unfortunately, the extraction of compact model parameters is a very knowledge and tool intensive process.
The model parameters found in many commercial and open-source PDKs are rather questionable and a serious problem
for circuit designers.
Bad models can result in tremendous follow-up costs due to re-design.

If you need help with model parameter extraction, consider reaching out to model extraction specialists.

### Who should I contact if I have problems, questions or feature requests?

You can post all questions, bugs or feature requests on the [OpenVAF Git repository](https://github.com/arpadbuermen/OpenVAF).
