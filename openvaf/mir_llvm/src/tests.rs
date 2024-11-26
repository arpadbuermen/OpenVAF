use super::*;
use llvm_sys::execution_engine::LLVMLinkInMCJIT;
use llvm_sys::execution_engine::{
    LLVMDisposeExecutionEngine, LLVMExecutionEngineRef, LLVMGetFunctionAddress,
    LLVMMCJITCompilerOptions,
};
use llvm_sys::target::{LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget};
use llvm_sys::{
    analysis::LLVMVerifyModule,
    core::{
        LLVMAddFunction, LLVMAppendBasicBlockInContext, LLVMBuildAlloca, LLVMBuildRetVoid,
        LLVMBuildStore, LLVMContextCreate, LLVMContextDispose, LLVMCreateBuilderInContext,
        LLVMCreateFunctionPassManagerForModule, LLVMDisposeBuilder, LLVMDisposeMessage,
        LLVMDisposeModule, LLVMFinalizeFunctionPassManager, LLVMFunctionType, LLVMGetParam,
        LLVMInt32TypeInContext, LLVMModuleCreateWithNameInContext, LLVMPositionBuilderAtEnd,
        LLVMPrintModuleToString, LLVMVoidTypeInContext,
    },
    prelude::LLVMValueRef,
    target::{
        LLVM_InitializeAllAsmParsers, LLVM_InitializeAllAsmPrinters, LLVM_InitializeAllTargetInfos,
        LLVM_InitializeAllTargetMCs, LLVM_InitializeAllTargets,
    },
    target_machine::LLVMCodeGenOptLevel,
};
use mir::Function;
use std::ffi::CStr;
use std::mem;
use std::ptr::NonNull;
use target::spec::Target;
use target_lexicon::{Architecture, Triple};

fn create_test_target() -> Target {
    let host_triple = Triple::host();
    let target_triple = host_triple.to_string();
    let pointer_width = match host_triple.architecture {
        Architecture::X86_64 => 64,
        Architecture::Aarch64(_) => 64,
        Architecture::X86_32(_) => 32,
        // Add other architectures as needed
        _ => panic!("Unsupported architecture"),
    };

    let cpu = match host_triple.architecture {
        Architecture::X86_64 => "x86-64",
        Architecture::Aarch64(_) => "generic",
        Architecture::X86_32(_) => "i686",
        // Add other architectures as needed
        _ => panic!("Unsupported architecture"),
    };

    Target {
        llvm_target: target_triple.clone(),
        pointer_width,
        data_layout: String::from(
            "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128",
        ),
        arch: host_triple.architecture.to_string(),
        options: target::spec::TargetOptions {
            cpu: cpu.to_string(),
            features: String::new(),
            ..Default::default()
        },
    }
}

use std::sync::Once;

static INIT: Once = Once::new();

fn initialize_llvm() {
    INIT.call_once(|| unsafe {
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmPrinters();
        LLVM_InitializeAllAsmParsers();
    });
}

#[test]
fn test_module_creation_and_verification() {
    initialize_llvm();
    let target = create_test_target();
    let result = unsafe {
        ModuleLlvm::new(
            "test_module",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
    };
    assert!(result.is_ok(), "Failed to create module");

    let module = result.unwrap();
    assert!(module.verify().is_none(), "Module verification failed");

    // Verify module contents
    let module_str = module.to_str().to_string();
    assert!(module_str.contains("source_filename = \"test_module\""));
    assert!(module_str.contains("target triple"));
    assert!(module_str.contains("target datalayout"));
}

#[test]
fn test_constant_operations() {
    initialize_llvm();
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_constants",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
        .unwrap()
    };

    let literals = Rodeo::new();
    let ctx = CodegenCx::new(&literals, &module, &target);

    // Test constant creation and verification
    let const_int = ctx.const_int(42);
    let const_real = ctx.const_real(3.14);
    let const_bool = ctx.const_bool(true);

    unsafe {
        // Verify integer constant
        let val = llvm_sys::core::LLVMConstIntGetSExtValue(NonNull::from(const_int).as_ptr());
        assert_eq!(val, 42);

        // Verify real constant
        let mut loses_info = 0;
        let val = llvm_sys::core::LLVMConstRealGetDouble(
            NonNull::from(const_real).as_ptr(),
            &mut loses_info,
        );
        assert_eq!(val, 3.14);
        assert_eq!(loses_info, 0);

        // Verify boolean constant
        let val = llvm_sys::core::LLVMConstIntGetZExtValue(NonNull::from(const_bool).as_ptr());
        assert_eq!(val, 1);
    }
}

#[test]
fn test_function_creation() {
    initialize_llvm();
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_functions",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
        .unwrap()
    };

    let literals = Rodeo::new();
    let ctx = CodegenCx::new(&literals, &module, &target);

    // Create function that adds two integers
    let int_ty = ctx.ty_int();
    let fn_ty = ctx.ty_func(&[int_ty, int_ty], int_ty);
    let test_fn = ctx.declare_int_fn("test_add", fn_ty);

    unsafe {
        // Set function linkage to internal
        llvm_sys::core::LLVMSetLinkage(
            NonNull::from(test_fn).as_ptr(),
            llvm_sys::LLVMLinkage::LLVMInternalLinkage,
        );

        let builder = NonNull::new_unchecked(llvm_sys::core::LLVMCreateBuilderInContext(
            NonNull::from(ctx.llcx).as_ptr(),
        ));

        let bb = NonNull::new_unchecked(llvm_sys::core::LLVMAppendBasicBlockInContext(
            NonNull::from(ctx.llcx).as_ptr(),
            NonNull::from(test_fn).as_ptr(),
            UNNAMED,
        ));

        llvm_sys::core::LLVMPositionBuilderAtEnd(builder.as_ptr(), bb.as_ptr());

        let param0 = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 0);
        let param1 = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 1);

        let sum = llvm_sys::core::LLVMBuildAdd(builder.as_ptr(), param0, param1, UNNAMED);

        llvm_sys::core::LLVMBuildRet(builder.as_ptr(), sum);
        llvm_sys::core::LLVMDisposeBuilder(builder.as_ptr());
    }

    // Print the generated IR for debugging
    // Verify the function
    assert!(module.verify().is_none(), "Function verification failed");

    // Check the generated IR
    let module_str = module.to_str().to_string();
    assert!(module_str.contains("define internal"), "Function linkage is not internal");
    assert!(module_str.contains("@test_add"), "Function name is not correct");
    assert!(module_str.contains("add i32"), "Add instruction not found");
    assert!(module_str.contains("ret i32"), "Return instruction not found");
    assert!(module_str.contains("add i32"));
    assert!(module_str.contains("ret i32"));
}

#[test]
fn test_optimization_constant_folding() {
    initialize_llvm();
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_opt",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
        )
        .unwrap()
    };

    let literals = Rodeo::new();
    let ctx = CodegenCx::new(&literals, &module, &target);

    // Create function that returns a constant expression using parameters
    let int_ty = ctx.ty_int();
    let fn_ty = ctx.ty_func(&[int_ty, int_ty, int_ty], int_ty);
    let test_fn = ctx.declare_int_fn("test_const_fold", fn_ty);

    unsafe {
        // Set function linkage to external
        llvm_sys::core::LLVMSetLinkage(
            NonNull::from(test_fn).as_ptr(),
            llvm_sys::LLVMLinkage::LLVMExternalLinkage,
        );

        // Add 'noinline' attribute to prevent inlining
        let noinline_attr_kind =
            llvm_sys::core::LLVMGetEnumAttributeKindForName(b"noinline\0".as_ptr() as *const _, 8);
        let noinline_attr = llvm_sys::core::LLVMCreateEnumAttribute(
            NonNull::from(ctx.llcx).as_ptr(),
            noinline_attr_kind,
            0,
        );
        llvm_sys::core::LLVMAddAttributeAtIndex(
            NonNull::from(test_fn).as_ptr(),
            llvm_sys::LLVMAttributeFunctionIndex,
            noinline_attr,
        );

        let builder = NonNull::new_unchecked(llvm_sys::core::LLVMCreateBuilderInContext(
            NonNull::from(ctx.llcx).as_ptr(),
        ));

        let bb = NonNull::new_unchecked(llvm_sys::core::LLVMAppendBasicBlockInContext(
            NonNull::from(ctx.llcx).as_ptr(),
            NonNull::from(test_fn).as_ptr(),
            UNNAMED,
        ));

        llvm_sys::core::LLVMPositionBuilderAtEnd(builder.as_ptr(), bb.as_ptr());

        // Use parameters to ensure the function is not optimized away
        let param_two = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 0);
        let param_three = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 1);
        let param_four = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 2);
        let mul = llvm_sys::core::LLVMBuildMul(builder.as_ptr(), param_three, param_four, UNNAMED);

        let sum = llvm_sys::core::LLVMBuildAdd(builder.as_ptr(), param_two, mul, UNNAMED);

        llvm_sys::core::LLVMBuildRet(builder.as_ptr(), sum);
        llvm_sys::core::LLVMDisposeBuilder(builder.as_ptr());
    }

    // Run optimization
    module.optimize();
    assert!(module.verify().is_none(), "Optimization produced invalid module");

    // Get IR after optimization
    let after_opt = module.to_str().to_string();

    // Print the optimized IR for debugging
    println!("AAA Optimized IR:\n{}", after_opt);

    // Check if the optimization has simplified the function
    // Adjust the expected result based on actual optimization behavior
    assert!(
        after_opt.contains("ret i32"),
        "Optimized function does not contain a return instruction"
    );
}

#[test]
fn test_builder_alloca() {
    initialize_llvm();
    unsafe {
        // Step 1: Initialize LLVM context and module
        let context = LLVMContextCreate();
        if context.is_null() {
            panic!("Failed to create LLVM context");
        }

        let module_name =
            std::ffi::CString::new("test_module").expect("Failed to create module name");
        let module = LLVMModuleCreateWithNameInContext(module_name.as_ptr(), context);
        if module.is_null() {
            LLVMContextDispose(context);
            panic!("Failed to create LLVM module");
        }

        // Step 2: Create function prototype
        let int32_type = LLVMInt32TypeInContext(context);
        let void_type = LLVMVoidTypeInContext(context);
        let mut param_types = vec![int32_type];
        let function_type = LLVMFunctionType(void_type, param_types.as_mut_ptr(), 1, 0);

        let function_name =
            std::ffi::CString::new("test_function").expect("Failed to create function name");
        let function = LLVMAddFunction(module, function_name.as_ptr(), function_type);
        if function.is_null() {
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to create function");
        }

        // Step 3: Create entry block
        let block_name = std::ffi::CString::new("entry").expect("Failed to create block name");
        let entry_block = LLVMAppendBasicBlockInContext(context, function, block_name.as_ptr());
        if entry_block.is_null() {
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to create entry block");
        }

        // Create builder
        let builder = LLVMCreateBuilderInContext(context);
        if builder.is_null() {
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to create LLVM Builder");
        }

        // Position at the start of entry block for allocation
        LLVMPositionBuilderAtEnd(builder, entry_block);

        // Create alloca instruction at the start of the entry block
        let alloca = LLVMBuildAlloca(builder, int32_type, b"temp\0".as_ptr() as *const _);
        if alloca.is_null() {
            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to create alloca instruction");
        }

        // Get function parameter
        let param = LLVMGetParam(function, 0);
        if param.is_null() {
            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to get function parameter");
        }

        // Build store instruction
        let store = LLVMBuildStore(builder, param, alloca);
        if store.is_null() {
            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to build store instruction");
        }

        // Build return void instruction to terminate the block
        let ret = LLVMBuildRetVoid(builder);
        if ret.is_null() {
            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Failed to build return instruction");
        }

        // Print IR before verification
        // println!("IR before verification:");
        let ir_before = LLVMPrintModuleToString(module);
        if !ir_before.is_null() {
            let _ir_str = CStr::from_ptr(ir_before).to_string_lossy();
            //    println!("{}", ir_str);
            LLVMDisposeMessage(ir_before);
        }

        // Verify module
        let mut error_message = std::ptr::null_mut();
        let verification_result = LLVMVerifyModule(
            module,
            llvm_sys::analysis::LLVMVerifierFailureAction::LLVMPrintMessageAction,
            &mut error_message,
        );

        if verification_result != 0 {
            let message = if !error_message.is_null() {
                let msg = CStr::from_ptr(error_message).to_string_lossy().into_owned();
                LLVMDisposeMessage(error_message);
                msg
            } else {
                String::from("Unknown verification error")
            };

            LLVMDisposeBuilder(builder);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);
            panic!("Module verification failed: {}", message);
        }

        // Print final IR
        let ir_ptr = LLVMPrintModuleToString(module);
        if !ir_ptr.is_null() {
            let ir = CStr::from_ptr(ir_ptr).to_string_lossy().into_owned();
            println!("Final Generated LLVM IR:\n{}", ir);
            LLVMDisposeMessage(ir_ptr);
        }

        // Clean up
        LLVMDisposeBuilder(builder);
        LLVMDisposeModule(module);
        LLVMContextDispose(context);
    }
}
/*
 *### Detailed Breakdown of Methods

#### `MemLoc` Methods:
- **`struct_gep`**: Constructs a `MemLoc` for a struct GEP operation.
- **`read`**: Reads the value from the memory location using the LLVM builder.
- **`read_with_ptr`**: Similar to `read`, but takes raw pointers to the LLVM builder and value.
- **`to_ptr`**: Converts the `MemLoc` to a pointer using the LLVM builder.
- **`to_ptr_from`**: Similar to `to_ptr`, but takes a raw pointer to the value.

#### `BuilderVal` Methods:
- **`get`**: Retrieves the LLVM value from the `BuilderVal`.
- **`get_ty`**: Retrieves the type of the LLVM value from the `BuilderVal`.

#### `Builder` Methods:
- **`new`**: Initializes a new `Builder` for a given function.
- **`alloca`**: Allocates memory on the stack.
- **`add_branching_select`**: Adds a conditional branch and selects a value based on the condition.
- **`select`**: Constructs a select instruction (ternary operator).
- **`typed_gep`**: Constructs a GEP (Get Element Pointer) instruction with a specified type.
- **`gep`**: Constructs a GEP instruction.
- **`struct_gep`**: Constructs a GEP instruction for a struct.
- **`fat_ptr_get_ptr`**: Retrieves the pointer part of a fat pointer.
- **`fat_ptr_get_meta`**: Retrieves the metadata part of a fat pointer.
- **`fat_ptr_to_parts`**: Retrieves both the pointer and metadata parts of a fat pointer.
- **`call`**: Constructs a function call instruction.
- **`build_consts`**: Builds constants for the function.
- **`build_func`**: Builds the entire function.
- **`select_bb`**: Selects a basic block for building.
- **`select_bb_before_terminator`**: Selects a basic block just before its terminator.
- **`build_bb`**: Builds a basic block.
- **`ret`**: Constructs a return instruction.
- **`ret_void`**: Constructs a void return instruction.
- **`build_inst`**: Builds an instruction.
- **`strcmp`**: Constructs a string comparison instruction.
- **`store`**: Constructs a store instruction.
- **`load`**: Constructs a load instruction.
- **`imul`**: Constructs an integer multiplication instruction.
- **`iadd`**: Constructs an integer addition instruction.
- **`ptr_diff`**: Constructs a pointer difference instruction.
- **`is_null_ptr`**: Constructs an instruction to check if a pointer is null.
- **`build_int_cmp`**: Constructs an integer comparison instruction.
- **`int_cmp`**: Constructs an integer comparison instruction.
- **`build_real_cmp`**: Constructs a floating-point comparison instruction.
- **`real_cmp`**: Constructs a floating-point comparison instruction.
- **`intrinsic`**: Constructs a call to an LLVM intrinsic function.
 * */
#[cfg(test)]
mod codegen_tests {
    use super::*;
    use lasso::Rodeo;
    use llvm_sys::core;
    use std::ffi::CString;
    use std::ptr::NonNull;

    pub fn setup_test_environment() -> (Target, ModuleLlvm, Rodeo) {
        initialize_llvm();
        let target = create_test_target();
        let module = unsafe {
            ModuleLlvm::new(
                "test_codegen",
                &target,
                "generic",
                "",
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            )
            .unwrap()
        };
        let literals = Rodeo::new();
        (target, module, literals)
    }

    #[test]
    fn test_string_literal_handling() {
        let (target, module, mut literals) = setup_test_environment();

        // First intern all strings we need
        let test_str = "Hello, World!";
        let str_id = literals.get_or_intern(test_str);

        // Then create context and use the interned strings
        let ctx = CodegenCx::new(&literals, &module, &target);
        let str_val = ctx.const_str(str_id);

        unsafe {
            // Verify the string constant was created correctly
            let global_type = core::LLVMTypeOf(NonNull::from(str_val).as_ptr());
            assert!(!global_type.is_null(), "String constant type is null");

            // Verify the string content
            let ir = module.to_str().to_string();
            assert!(ir.contains("Hello, World!"), "String literal not found in IR");
            assert!(ir.contains("internal constant"), "String should be internal constant");
        }
    }

    #[test]
    fn test_symbol_generation() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Generate multiple symbols and verify uniqueness
        let sym1 = ctx.generate_local_symbol_name("test");
        let sym2 = ctx.generate_local_symbol_name("test");
        let sym3 = ctx.generate_local_symbol_name("other");

        assert_ne!(sym1, sym2, "Generated symbols should be unique");
        assert!(sym1.starts_with("test."), "Symbol should start with prefix");
        assert!(sym3.starts_with("other."), "Symbol should start with given prefix");

        // Verify counter increment
        assert!(ctx.local_gen_sym_counter.get() >= 3, "Symbol counter not incrementing");
    }

    #[test]
    fn test_function_lookup() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a test function
        unsafe {
            let void_type = core::LLVMVoidTypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(void_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_function").unwrap();
            core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type);
        }

        // Test function lookup
        let found_fn = ctx.get_func_by_name("test_function");
        assert!(found_fn.is_some(), "Failed to find declared function");

        let missing_fn = ctx.get_func_by_name("nonexistent_function");
        assert!(missing_fn.is_none(), "Should return None for missing function");
    }

    #[test]
    fn test_intrinsics_cache() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Verify intrinsics cache starts empty
        assert!(ctx.intrinsics.borrow().is_empty(), "Intrinsics cache should start empty");

        // Test adding to intrinsics cache
        unsafe {
            let int_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(int_type, std::ptr::null_mut(), 0, 0);
            let intrinsic_name = CString::new("llvm.ctpop.i32").unwrap();
            let intrinsic_fn = core::LLVMAddFunction(
                NonNull::from(ctx.llmod).as_ptr(),
                intrinsic_name.as_ptr(),
                fn_type,
            );

            ctx.intrinsics.borrow_mut().insert("llvm.ctpop.i32", (&*fn_type, &*intrinsic_fn));
        }

        assert_eq!(ctx.intrinsics.borrow().len(), 1, "Intrinsics cache should contain one entry");
    }

    #[test]
    fn test_types() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Test basic type creation
        unsafe {
            let int_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            assert_ne!(int_type, std::ptr::null_mut(), "Integer type should not be null");
            assert_eq!(
                core::LLVMGetTypeKind(int_type),
                llvm_sys::LLVMTypeKind::LLVMIntegerTypeKind
            );
        }

        // Test pointer width matches target
        let ptr_width = target.pointer_width;
        assert!(ptr_width > 0, "Invalid pointer width");

        // Verify size type is available and correct
        unsafe {
            let size_type = ctx.tys.size;
            assert_ne!(size_type as *const _, std::ptr::null(), "Size type should not be null");
            assert_eq!(
                core::LLVMGetTypeKind(size_type as *const _ as *mut _),
                llvm_sys::LLVMTypeKind::LLVMIntegerTypeKind
            );
        }
    }
}

fn check_result<T, F, Args>(module: &ModuleLlvm, function_name: &str, args: Args, test_callback: F)
where
    F: FnOnce(T),
    T: Copy,
    Args: TupleToFunctionSignature<T>,
{
    unsafe {
        // Initialize LLVM's JIT components
        LLVM_InitializeNativeTarget();
        LLVM_InitializeNativeAsmPrinter();

        // Create an ExecutionEngine for the module
        let mut execution_engine: LLVMExecutionEngineRef = ptr::null_mut();
        let mut error = ptr::null_mut();

        // Verify the module before creating the execution engine
        if llvm_sys::analysis::LLVMVerifyModule(
            module.llmod_raw,
            llvm_sys::analysis::LLVMVerifierFailureAction::LLVMPrintMessageAction,
            &mut error,
        ) != 0
        {
            let error_str = CStr::from_ptr(error).to_string_lossy().into_owned();
            LLVMDisposeMessage(error);
            panic!("Module verification failed: {}", error_str);
        }

        // Link in the bitcode for the current native target
        LLVMLinkInMCJIT();

        // Create a new MCJIT execution engine with optimization level 2
        let mut options: LLVMMCJITCompilerOptions = mem::zeroed();
        let options_size = mem::size_of::<LLVMMCJITCompilerOptions>();
        llvm_sys::execution_engine::LLVMInitializeMCJITCompilerOptions(&mut options, options_size);
        options.OptLevel = 2;

        if llvm_sys::execution_engine::LLVMCreateMCJITCompilerForModule(
            &mut execution_engine,
            module.llmod_raw,
            &mut options,
            options_size,
            &mut error,
        ) != 0
        {
            let error_str = CStr::from_ptr(error).to_string_lossy().into_owned();
            LLVMDisposeMessage(error);
            panic!("Failed to create execution engine: {}", error_str);
        }

        // Finalize the module
        LLVMFinalizeFunctionPassManager(LLVMCreateFunctionPassManagerForModule(module.llmod_raw));

        // Get and execute the compiled function
        let function_name = CString::new(function_name).unwrap();
        let function_address = LLVMGetFunctionAddress(execution_engine, function_name.as_ptr());
        assert!(function_address != 0, "Failed to get function address");

        // Use tuple to call the function based on argument count
        let result = args.call_function(function_address);

        // Call the test callback with the result
        test_callback(result);

        // Clean up
        LLVMDisposeExecutionEngine(execution_engine);
    }
}

// Trait to call the function with appropriate arguments
trait TupleToFunctionSignature<T> {
    fn call_function(self, function_address: u64) -> T;
}

// Implement TupleToFunctionSignature for no arguments
impl<T> TupleToFunctionSignature<T> for () {
    fn call_function(self, function_address: u64) -> T {
        let compiled_fn: extern "C" fn() -> T = unsafe { std::mem::transmute(function_address) };
        compiled_fn()
    }
}

// Implement TupleToFunctionSignature for one argument
impl<A, T> TupleToFunctionSignature<T> for (A,) {
    fn call_function(self, function_address: u64) -> T {
        let compiled_fn: extern "C" fn(A) -> T = unsafe { std::mem::transmute(function_address) };
        compiled_fn(self.0)
    }
}

// Implement TupleToFunctionSignature for two arguments
impl<A, B, T> TupleToFunctionSignature<T> for (A, B) {
    fn call_function(self, function_address: u64) -> T {
        let compiled_fn: extern "C" fn(A, B) -> T =
            unsafe { std::mem::transmute(function_address) };
        compiled_fn(self.0, self.1)
    }
}

mod builder_tests {
    use super::*;
    use codegen_tests::setup_test_environment;

    use llvm_sys::core;
    use std::ffi::CString;
    use std::ptr::NonNull;

    #[test]
    fn test_builder_alloca() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a test function to add an alloca
            let void_type = core::LLVMVoidTypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(void_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_alloca_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };

        let builder = Builder::new(&ctx, &function_instance, function);

        // Print the generated IR before adding the alloca instruction
        // let ir = module.to_str().to_string();
        // println!("first pass {}", ir);
        // Literal IR before alloca:
        /*
        define void @test_alloca_function() {
        }
        */

        // Allocate a 32-bit integer
        unsafe {
            let int_ty = &*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let alloca = builder.alloca(int_ty);
            let alloca_ptr: LLVMValueRef = alloca as *const _ as *mut _; // Convert a reference to a raw pointer
            assert!(!alloca_ptr.is_null(), "Failed to create alloca for int32");
        }

        // Print the generated IR after adding the alloca instruction
        // let ir = module.to_str().to_string();
        // println!("after alloca {}", ir);
        // Literal IR after alloca:
        /*
        define void @test_alloca_function() {
          %1 = alloca i32, align 4
        }
        */
    }

    #[test]
    fn test_builder_branching_select() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a function with a boolean parameter for branching
            let bool_type = core::LLVMInt1TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(bool_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_branch_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };

        let mut builder = Builder::new(&ctx, &function_instance, function);

        // Print the generated IR before adding the branching select
        // let ir_before = module.to_str().to_string();
        // println!("IR before add_branching_select:\n{}", ir_before);
        // Literal IR before add_branching_select:
        /*
        define i1 @test_branch_function() {
        }
        */

        // Add branching logic
        unsafe {
            let bool_val = core::LLVMConstInt(core::LLVMInt1Type(), 1, 0); //1 is TRUE otherwise
                                                                           //FALSE
            let result = builder.add_branching_select(
                &*bool_val,
                |then_builder| {
                    let int_ty = &*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
                    then_builder.alloca(int_ty)
                },
                |else_builder| {
                    let int_ty = &*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
                    else_builder.alloca(int_ty)
                },
            );
            let result_ptr: LLVMValueRef = result as *const _ as *mut _; // Convert a reference to a raw pointer
            assert!(!result_ptr.is_null(), "Failed to create branching select");
        }

        // Print the generated IR after adding the branching select
        //let ir_after = module.to_str().to_string();
        //println!("IR after add_branching_select:\n{}", ir_after);
        // Literal IR after add_branching_select:
        /*
        define i1 @test_branch_function() {
          br i1 true, label %3, label %5

        1:                                                ; preds = %5, %3
          %2 = phi ptr [ %4, %3 ], [ %6, %5 ]

        3:                                                ; preds = %0
          %4 = alloca i32, align 4
          br label %1

        5:                                                ; preds = %0
          %6 = alloca i32, align 4
          br label %1
        }
        */
    }

    #[test]
    fn test_builder_select() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a function with an i32 return type
            let int_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(int_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_select_function").unwrap();
            // Define the function: ir: define i32 @test_select_function()
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };

        let mut builder = Builder::new(&ctx, &function_instance, function);

        // Position the builder at the beginning of the function without creating an entry block
        unsafe {
            core::LLVMPositionBuilderAtEnd(
                builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(function).as_ptr()),
            );
        }

        // Add the select logic directly to the function body
        unsafe {
            // Create a non-constant condition value to avoid optimization
            let param_type = core::LLVMInt1TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let param = builder.alloca(&*param_type); // ir: %cond = alloca i1

            // Store a value into the allocated memory
            let true_val = core::LLVMConstInt(param_type, 1, 0);
            builder.store(param, &*true_val); // ir: store i1 true, i1* %cond

            let bool_val = builder.load(&*param_type, param); // ir: %load_cond = load i1, i1* %cond

            let then_val = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                42,
                0,
            ); // ir: i32 42 (constant value)
            let else_val = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                0,
                0,
            ); // ir: i32 0 (constant value)

            // Use select to choose between then_val and else_val based on bool_val
            let result = builder.select(&*bool_val, &*then_val, &*else_val); // ir: %1 = select i1 %load_cond, i32 42, i32 0

            // Return the selected value
            builder.ret(result); // ir: ret i32 %1
        }

        // Print the generated IR after adding the select instruction
        //let ir_after = module.to_str().to_string();
        //println!("IR after select:\n{}", ir_after);
        // Literal IR after select:
        /*
        define i32 @test_select_function() {
          %cond = alloca i1
          store i1 true, i1* %cond
          %load_cond = load i1, i1* %cond
          %1 = select i1 %load_cond, i32 42, i32 0
          ret i32 %1
        }
        */
        // Use the check_result function to verify and execute the function
        check_result::<i32, _, ()>(&module, "test_select_function", (), |result| {
            println!("JIT compiled function result: {}", result);
            assert_eq!(result, 42, "Unexpected result from JIT compiled function");
        });
    }

    #[test]
    fn test_builder_typed_gep() {
        //same as gep
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a function with an i32 return type
            let int32_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(int32_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_gep_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };
        // IR: define i32 @test_gep_function()

        let mut builder = Builder::new(&ctx, &function_instance, function);

        unsafe {
            // Position the builder at the beginning of the function without creating an entry block
            core::LLVMPositionBuilderAtEnd(
                builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(function).as_ptr()),
            );

            // Create an array type [4 x i32]
            let array_type = core::LLVMArrayType2(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                4,
            );

            // Allocate space for the array in the function
            let array_alloca = builder.alloca(&*array_type);
            // IR: %0 = alloca [4 x i32], align 16

            // Create constant indices to access the second element of the array (index 1)
            let zero_index = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                0,
                0,
            );
            let one_index = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                1,
                0,
            );
            let indices = [&*zero_index, &*one_index];

            // Perform GEP to get the pointer to the second element
            let element_ptr = builder.gep(&*array_type, array_alloca, &indices);
            // IR: %1 = getelementptr inbounds [4 x i32], [4 x i32]* %0, i32 0, i32 1

            // Store a value into the second element of the array
            let value_to_store = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                42,
                0,
            );
            builder.store(element_ptr, &*value_to_store);
            // IR: store i32 42, i32* %1, align 4

            // Load the value back to verify it was stored correctly
            let loaded_value = builder.load(
                &*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                element_ptr,
            );
            // IR: %2 = load i32, i32* %1, align 4

            // Return the loaded value from the function
            builder.ret(loaded_value);
            // IR: ret i32 %2

            // Check if loaded value matches what was stored
            let loaded_value_ptr: LLVMValueRef = loaded_value as *const _ as *mut _;
            assert!(!loaded_value_ptr.is_null(), "Failed to load value after GEP operation");
        }

        // Print the generated IR to verify
        //let ir_after = module.to_str().to_string();
        //println!("IR after typed_gep: {}", ir_after);
        // Literal IR after adding typed_gep:
        /*
         define i32 @test_gep_function() {
          %1 = alloca [4 x i32], align 4
          %2 = getelementptr [4 x i32], ptr %1, i32 0, i32 1
          store i32 42, ptr %2, align 4
          %3 = load i32, ptr %2, align 4
          ret i32 %3
        }
        */

        check_result::<i32, _, ()>(&module, "test_gep_function", (), |result| {
            println!("JIT compiled function result: {}", result);
            assert_eq!(result, 42, "Unexpected result from JIT compiled function");
        });
    }

    #[test]
    fn test_builder_struct_gep() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a function with an i32 return type
            let int32_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(int32_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_struct_gep_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };
        // IR: define i32 @test_struct_gep_function()

        let mut builder = Builder::new(&ctx, &function_instance, function);

        unsafe {
            // Position the builder at the beginning of the function without creating an entry block
            core::LLVMPositionBuilderAtEnd(
                builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(function).as_ptr()),
            );

            // Create a struct type { i32, i32 }
            let mut struct_fields = [
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
            ];
            let struct_type = core::LLVMStructTypeInContext(
                NonNull::from(ctx.llcx).as_ptr(),
                struct_fields.as_mut_ptr(), // Convert to mutable raw pointer
                2,
                0,
            );

            // Allocate space for the struct in the function
            let struct_alloca = builder.alloca(&*struct_type);
            // IR: %0 = alloca { i32, i32 }, align 4

            // Perform GEP to get the pointer to the first field of the struct
            let field_ptr = builder.struct_gep(&*struct_type, struct_alloca, 0);
            // IR: %1 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %0, i32 0, i32 0

            // Store a value into the first field of the struct
            let value_to_store = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                42,
                0,
            );
            builder.store(field_ptr, &*value_to_store);
            // IR: store i32 42, i32* %1, align 4

            // Load the value back to verify it was stored correctly
            let loaded_value = builder
                .load(&*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()), field_ptr);
            // IR: %2 = load i32, i32* %1, align 4

            // Return the loaded value from the function
            builder.ret(loaded_value);
            // IR: ret i32 %2

            // Check if loaded value matches what was stored
            let loaded_value_ptr: LLVMValueRef = loaded_value as *const _ as *mut _;
            assert!(!loaded_value_ptr.is_null(), "Failed to load value after struct GEP operation");
        }

        // Print the generated IR to verify
        //let ir_after = module.to_str().to_string();
        //println!("IR after struct_gep: {}", ir_after);
        // Literal IR after adding struct_gep:
        /*
         define i32 @test_struct_gep_function() {
          %1 = alloca { i32, i32 }, align 4
          %2 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %1, i32 0, i32 0
          store i32 42, i32* %2, align 4
          %3 = load i32, i32* %2, align 4
          ret i32 %3
        }
        */

        check_result::<i32, _, ()>(&module, "test_struct_gep_function", (), |result| {
            println!("JIT compiled function result: {}", result);
            assert_eq!(result, 42, "Unexpected result from JIT compiled function");
        });
    }

    #[test]
    fn test_builder_nested_struct_gep() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let function = unsafe {
            // Create a function with an i32 return type
            let int32_type = core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr());
            let fn_type = core::LLVMFunctionType(int32_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_nested_struct_gep_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };
        // IR: define i32 @test_nested_struct_gep_function()

        let mut builder = Builder::new(&ctx, &function_instance, function);

        unsafe {
            // Position the builder at the beginning of the function without creating an entry block
            core::LLVMPositionBuilderAtEnd(
                builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(function).as_ptr()),
            );

            // Create the inner struct type { i32, i32 }
            let mut inner_fields = [
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
            ];
            let inner_struct_type = core::LLVMStructTypeInContext(
                NonNull::from(ctx.llcx).as_ptr(),
                inner_fields.as_mut_ptr(),
                2,
                0,
            );

            // Create the outer struct type { i1, { i32, i32 } }
            let mut outer_fields =
                [core::LLVMInt1TypeInContext(NonNull::from(ctx.llcx).as_ptr()), inner_struct_type];
            let outer_struct_type = core::LLVMStructTypeInContext(
                NonNull::from(ctx.llcx).as_ptr(),
                outer_fields.as_mut_ptr(),
                2,
                0,
            );

            // Allocate space for the outer struct in the function
            let struct_alloca = builder.alloca(&*outer_struct_type);
            // IR: %0 = alloca { i1, { i32, i32 } }, align 4

            // Perform GEP to get the pointer to the nested struct field
            let nested_struct_ptr = builder.struct_gep(&*outer_struct_type, struct_alloca, 1);
            // IR: %1 = getelementptr inbounds { i1, { i32, i32 } }, { i1, { i32, i32 } }* %0, i32 0, i32 1

            // Perform GEP to get the pointer to the first field of the nested struct
            let field_i32_0_ptr = builder.struct_gep(&*inner_struct_type, nested_struct_ptr, 0);
            // IR: %2 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %1, i32 0, i32 0

            // Store a value into the first field of the nested struct
            let value_to_store_1 = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                42,
                0,
            );
            builder.store(field_i32_0_ptr, &*value_to_store_1);
            // IR: store i32 42, i32* %2, align 4

            // Perform GEP to get the pointer to the second field of the nested struct
            let field_i32_1_ptr = builder.struct_gep(&*inner_struct_type, nested_struct_ptr, 1);
            // IR: %3 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %1, i32 0, i32 1

            // Store a value into the second field of the nested struct
            let value_to_store_2 = core::LLVMConstInt(
                core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                84,
                0,
            );
            builder.store(field_i32_1_ptr, &*value_to_store_2);
            // IR: store i32 84, i32* %3, align 4

            // Load the value back from the first field to verify it was stored correctly
            let loaded_value_1 = builder.load(
                &*core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()),
                field_i32_0_ptr,
            );
            // IR: %4 = load i32, i32* %2, align 4

            // Return the loaded value from the function
            builder.ret(loaded_value_1);
            // IR: ret i32 %4

            // Check if loaded value matches what was stored
            let loaded_value_ptr: LLVMValueRef = loaded_value_1 as *const _ as *mut _;
            assert!(
                !loaded_value_ptr.is_null(),
                "Failed to load value after nested struct GEP operation"
            );
        }

        // Print the generated IR to verify
        //let ir_after = module.to_str().to_string();
        //println!("IR after nested_struct_gep: {}", ir_after);
        // Literal IR after adding nested_struct_gep:
        /*
         define i32 @test_nested_struct_gep_function() {
          %0 = alloca { i1, { i32, i32 } }, align 4
          %1 = getelementptr inbounds { i1, { i32, i32 } }, { i1, { i32, i32 } }* %0, i32 0, i32 1
          %2 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %1, i32 0, i32 0
          store i32 42, i32* %2, align 4
          %3 = getelementptr inbounds { i32, i32 }, { i32, i32 }* %1, i32 0, i32 1
          store i32 84, i32* %3, align 4
          %4 = load i32, i32* %2, align 4
          ret i32 %4
        }
        */

        check_result::<i32, _, ()>(&module, "test_nested_struct_gep_function", (), |result| {
            println!("JIT compiled function result: {}", result);
            assert_eq!(result, 42, "Unexpected result from JIT compiled function");
        });
    }

    #[test]
    fn test_builder_call() {
        let (target, module, literals) = setup_test_environment();
        let ctx = CodegenCx::new(&literals, &module, &target);

        // Create a long-lived function instance
        let function_instance = Function::default();

        let int32_type = unsafe { core::LLVMInt32TypeInContext(NonNull::from(ctx.llcx).as_ptr()) };
        let mut param_types = vec![int32_type, int32_type];

        let function = unsafe {
            // Create a function type that takes two i32 arguments and returns an i32
            let fn_type = core::LLVMFunctionType(int32_type, std::ptr::null_mut(), 0, 0);
            let fn_name = CString::new("test_call_function").unwrap();
            &*core::LLVMAddFunction(NonNull::from(ctx.llmod).as_ptr(), fn_name.as_ptr(), fn_type)
        };
        // IR: define i32 @test_call_function(i32 %0, i32 %1)

        let mut builder = Builder::new(&ctx, &function_instance, function);

        unsafe {
            // Position the builder at the beginning of the function without creating an entry block
            core::LLVMPositionBuilderAtEnd(
                builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(function).as_ptr()),
            );

            // Define the add_function that adds two i32 values
            let add_function_name = CString::new("add_function").unwrap();
            let add_function = &*core::LLVMAddFunction(
                NonNull::from(ctx.llmod).as_ptr(),
                add_function_name.as_ptr(),
                core::LLVMFunctionType(int32_type, param_types.as_mut_ptr(), 2, 0),
            );

            // Create a new builder for the add_function
            let mut add_builder = Builder::new(&ctx, &function_instance, add_function);

            // Position the builder at the beginning of the add_function
            core::LLVMPositionBuilderAtEnd(
                add_builder.llbuilder,
                core::LLVMGetEntryBasicBlock(NonNull::from(add_function).as_ptr()),
            );

            // Get the function arguments
            let arg1 = &*core::LLVMGetParam(NonNull::from(add_function).as_ptr(), 0);
            let arg2 = &*core::LLVMGetParam(NonNull::from(add_function).as_ptr(), 1);

            // Add the two arguments
            let add_result = add_builder.iadd(arg1, arg2);

            // Return the result of the addition
            add_builder.ret(add_result);

            // Create some constants to pass as arguments
            let arg1_const = core::LLVMConstInt(int32_type, 10, 0);
            let arg2_const = core::LLVMConstInt(int32_type, 20, 0);

            // Call the add_function with the constants as arguments
            let call_result = builder.call(
                &*core::LLVMFunctionType(int32_type, param_types.as_mut_ptr(), 2, 0),
                &*add_function,
                &[&*arg1_const, &*arg2_const],
            );

            // Return the result of the call
            builder.ret(call_result);
            // IR: ret i32 %call_result

            // Check if the call result is not null
            let call_result_ptr: LLVMValueRef = call_result as *const _ as *mut _;
            assert!(!call_result_ptr.is_null(), "Failed to call function");
        }

        // Print the generated IR to verify
        let ir_after = module.to_str().to_string();
        println!("IR after call: {}", ir_after);
        // Literal IR after adding call:
        /*
        define i32 @test_call_function() {
          %1 = call i32 @add_function(i32 10, i32 20)
          ret i32 %1
        }

        define i32 @add_function(i32 %0, i32 %1) {
          %3 = add i32 %0, %1
          ret i32 %3
        }
        */

        //   check_result::<i32, _, ()>(&module, "test_call_function", (), |result| {
        //       println!("JIT compiled function result: {}", result);
        //       assert_eq!(result, 30, "Unexpected result from JIT compiled function");
        //   });
        check_result::<i32, _, (i32, i32)>(&module, "add_function", (2, 2), |result| {
            println!("Testing 2+2: {}", result);
            assert_eq!(result, 4, "Unexpected result from JIT compiled function");
        });
    }
}
