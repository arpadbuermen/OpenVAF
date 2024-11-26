use super::LinkerFlavor;
use crate::spec::TargetOptions;

pub fn opts() -> TargetOptions {
    TargetOptions {
        linker_flavor: LinkerFlavor::Ld64,
        is_like_osx: true,
        ..TargetOptions::default()
    }
}
