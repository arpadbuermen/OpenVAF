use crate::spec::{LinkerFlavor, Target};

pub fn target() -> Target {
    let mut base = super::windows_base::opts_windows_base();
    base.cpu = "x86-64".to_string();
    base.linker_flavor = LinkerFlavor::Ld;
    base.pre_link_args.insert(LinkerFlavor::Ld, vec!["-m64".to_string()]);

    Target {
        llvm_target: "x86_64-pc-windows-gnu".to_string(),
        arch: "x86_64".to_string(),
        data_layout: "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
            .to_string(),
        options: base,
        pointer_width: 64,
    }
}

