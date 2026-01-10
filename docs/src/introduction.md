# Introduction

## Circuit Simulation

Circuit simulators play a critical role in the design of electrical circuits.
Accurate simulations enable circuit designers to validate circuit behavior before actual fabrication happens,
potentially saving **significant re-design costs.**
The simulation of a circuit critically depends on the so-called compact models and therefore:

* The **accuracy of the compact-model equations**
* The quality of the **model parameters**

## Compact Models

Compact models predict the device **terminal characteristics** by means of computationally inexpensive equations.
With increasingly advanced technologies, compact models have been **growing significantly in complexity**.
At the same time an increasingly diverse set of technologies is offered to designers, requiring **specific compact models for each kind of electron device**.

The complexity of compact models has made the manual integration into simulators **a tedious, error-prone and therefore expensive** task.
One reason for this is that not only the model equations have to be implemented, but also their symbolic derivatives.
Numeric derivatives are not an option because they are orders of magnitude slower to compute than analytical derivatives and can introduce convergence problems due to inaccuracies.
It is not uncommon - even in commercial tools - to find model **implementation bugs** or to observe **convergence problems** that result
from incorrectly implemented derivatives.
Some simulators with no or limited Verilog-A integration **do not implement certain compact-models and can therefore not be used to simulate some processes at all**.

Manually implemented compact models may **differ between simulators** since EDA vendors often rename parameters or alter particular model equations.
Due to these simulator specific peculiarities, PDKs can usually only be used by a few specific simulators.

## Verilog-A

Verilog-A has been developed to address these problems and has become the [de-facto standard](https://si2.org/standard-models/) for developing and distributing compact models.
It allows implementing compact models via a **simulator independent** and standardized language.
**Verilog-A compilers** can translate these models to machine code and allow simulators to use these models **without manually implementing them**.

Verilog-A enables...

* **model development and customization** by allowing to quickly modify the model equations without having to worry about model implementation details.
* implementing **behavioral or data-driven models**, or even entire circuits.
* inherent **portability between simulators** for both models and PDKs that would not be possible with traditional netlist-based formats.

Model development and customization is necessary for advanced technologies and applications, for example quantum computing,
where existing models cannot provide satisfactory results and must be adjusted.
It also enables research and development.
