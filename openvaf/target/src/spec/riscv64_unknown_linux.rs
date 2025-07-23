use crate::spec::{linux_base, Target};

pub fn target() -> Target {
    Target {
        llvm_target: "riscv64-unknown-linux-gnu".to_string(),
        pointer_width: 64,
        data_layout: "e-m:e-p:64:64-i64:64-i128:128-n32:64-S128".to_string(),
        arch: "riscv64".to_string(),
        options: linux_base::opts(),
    }
}
