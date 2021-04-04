use openvaf_targets::spec::Target;
use crate::sim::Simulator;

pub mod sim;
pub use openvaf_targets as bin_targets;

pub struct CompilationTarget {
    native_target: Target,
    simulator: Simulator,
}