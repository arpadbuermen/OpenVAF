use crate::spec::{LinkArgs, LinkerFlavor, TargetOptions};

/// Base options for all Windows targets, excluding MSVC-specific arguments.
pub fn opts_windows_base(flavor: LinkerFlavor) -> TargetOptions {
    let pre_link_args = LinkArgs::new();
    let post_link_args = LinkArgs::new();

    TargetOptions {
        is_like_windows: true,
        linker_flavor: flavor,
        pre_link_args,
        post_link_args,
        ..Default::default()
    }
}
