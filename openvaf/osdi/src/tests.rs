use std::path::Path;

use camino::Utf8Path;
use llvm_sys::target_machine::LLVMCodeGenOptLevel;
use mir_llvm::LLVMBackend;
use paths::AbsPathBuf;
use sim_back::CompilationDB;
use stdx::SKIP_HOST_TESTS;
use target::spec::Target;

mod integration;
mod sourcegen;

fn test_compile(root_file: &Path) {
    let root_file = AbsPathBuf::assert(root_file.canonicalize().unwrap());
    let db = CompilationDB::new(root_file, &[], &[], &[]).unwrap();
    let modules = db.collect_modules().unwrap();
    let target = Target::host_target().unwrap();
    let back = LLVMBackend::new(&[], &target, "native".to_owned(), &[]);
    let emit = !stdx::IS_CI;
    crate::compile(
        &db,
        &modules,
        Utf8Path::new("foo.o"),
        &target,
        &back,
        emit,
        LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
    );
}

#[test]
pub fn generate_integration_tests() {
    if SKIP_HOST_TESTS {
        return;
    }
    let tests = collect_integration_tests();
    let file = project_root().join("openvaf/osdi/src/tests/integration.rs");
    let test_impl = tests.into_iter().map(|(test_name, _)| {
        let test_case = format_ident!("{}", test_name.to_lowercase());
        let root_file_name = format!("{}.va", test_name.to_lowercase());

        quote! {
            #[test]
            fn #test_case(){
                if skip_slow_tests(){
                    return
                }

                let root_file = project_root().join("integration_tests").join(#test_name).join(#root_file_name);
                super::test_compile(&root_file);
            }
        }

    });

    let header = "
        use sourcegen::{skip_slow_tests, project_root};
    ";

    let file_string = quote!(
        #(#test_impl)*
    );
    let file_string = format!("{}\n{}", header, file_string);
    let file_string = add_preamble("generate_integration_tests", file_string);
    let file_string = reformat(file_string);
    ensure_file_contents(&file, &file_string);
}
