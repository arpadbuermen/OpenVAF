
use super::*;
use llvm_sys::target_machine::LLVMCodeGenOptLevel;
use std::path::PathBuf;
use target::spec::Target;

fn create_test_target() -> Target {
    Target {
        llvm_target: String::from("x86_64-unknown-linux-gnu"),
        pointer_width: 64,
        data_layout: String::from("e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"),
        options: target::spec::TargetOptions {
            cpu: String::from("x86-64"),
            features: String::new(),
            ..Default::default()
        },
    }
}

#[test]
fn test_module_creation() {
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
}

#[test]
fn test_context_operations() {
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_context",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
        .unwrap()
    };

    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };

    // Test type creation
    let double_ty = ctx.ty_double();
    let int_ty = ctx.ty_int();
    let bool_ty = ctx.ty_bool();
    
    assert!(!double_ty.is_null());
    assert!(!int_ty.is_null());
    assert!(!bool_ty.is_null());

    // Test constant creation
    let const_int = ctx.const_int(42);
    let const_real = ctx.const_real(3.14);
    let const_bool = ctx.const_bool(true);

    assert!(!const_int.is_null());
    assert!(!const_real.is_null());
    assert!(!const_bool.is_null());
}

#[test]
fn test_builder_operations() {
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_builder",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
        .unwrap()
    };

    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };

    // Create a test function
    let fn_ty = ctx.ty_func(&[], ctx.ty_void());
    let test_fn = ctx.declare_int_fn("test_function", fn_ty);
    
    // Create a builder
    let builder = unsafe {
        let builder = llvm_sys::core::LLVMCreateBuilderInContext(ctx.llcx as *mut _);
        let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
            ctx.llcx as *mut _,
            test_fn as *mut _,
            UNNAMED,
        );
        llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
        builder
    };

    assert!(!builder.is_null());

    unsafe {
        llvm_sys::core::LLVMDisposeBuilder(builder);
    }
}

#[test]
fn test_optimization() {
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

    // Add a simple function to optimize
    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };
    
    let fn_ty = ctx.ty_func(&[], ctx.ty_int());
    let test_fn = ctx.declare_int_fn("test_opt_function", fn_ty);
    
    unsafe {
        let builder = llvm_sys::core::LLVMCreateBuilderInContext(ctx.llcx as *mut _);
        let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
            ctx.llcx as *mut _,
            test_fn as *mut _,
            UNNAMED,
        );
        llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
        
        // Build: return 2 + 3
        let two = ctx.const_int(2);
        let three = ctx.const_int(3);
        let sum = llvm_sys::core::LLVMBuildAdd(builder, two as *mut _, three as *mut _, UNNAMED);
        llvm_sys::core::LLVMBuildRet(builder, sum);
        
        llvm_sys::core::LLVMDisposeBuilder(builder);
    }

    // Run optimization
    module.optimize();
    assert!(module.verify().is_none(), "Optimization produced invalid module");
}

#[test]
fn test_object_emission() {
    let target = create_test_target();
    let module = unsafe {
        ModuleLlvm::new(
            "test_emit",
            &target,
            "generic",
            "",
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
        )
        .unwrap()
    };

    // Add a simple function
    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };
    
    let fn_ty = ctx.ty_func(&[], ctx.ty_void());
    let test_fn = ctx.declare_int_fn("test_emit_function", fn_ty);
    
    unsafe {
        let builder = llvm_sys::core::LLVMCreateBuilderInContext(ctx.llcx as *mut _);
        let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
            ctx.llcx as *mut _,
            test_fn as *mut _,
            UNNAMED,
        );
        llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
        llvm_sys::core::LLVMBuildRetVoid(builder);
        llvm_sys::core::LLVMDisposeBuilder(builder);
    }

    // Emit object file
    let output_path = PathBuf::from("test_output.o");
    let result = module.emit_object(&output_path);
    assert!(result.is_ok(), "Failed to emit object file");

    // Cleanup
    std::fs::remove_file(output_path).unwrap();
}
