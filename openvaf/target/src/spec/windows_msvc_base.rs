use crate::spec::{LinkerFlavor, TargetOptions};

/// MSVC-specific Windows target options, extending the base Windows options.
pub fn opts() -> TargetOptions {
    let mut base = super::windows_base::opts_windows_base(LinkerFlavor::Msvc);

    // Suppress the verbose logo and authorship debugging output, which would needlessly
    // clog any log files.
    // Add MSVC-specific linker arguments like `/NOLOGO` and `msvcrt.lib`
    base.pre_link_args.entry(LinkerFlavor::Msvc).or_default().push("/NOLOGO".to_string());

    base.post_link_args.entry(LinkerFlavor::Msvc).or_default().push("msvcrt.lib".to_string());

    base
}
