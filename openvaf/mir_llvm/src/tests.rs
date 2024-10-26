
use super::*;
use llvm_sys::target_machine::LLVMCodeGenOptLevel;
use std::path::PathBuf;
use target::spec::Target;

fn create_test_target() -> Target {
    Target {
        llvm_target: String::from("x86_64-unknown-linux-gnu"),
        pointer_width: 64,
        data_layout: String::from("e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"),
        arch: target::spec::Architecture::X86_64,
        options: target::spec::TargetOptions {
            cpu: String::from("x86-64"),
            features: String::new(),
            ..Default::default()
        },
    }
}

#[test]
fn test_module_creation_and_verification() {
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

    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };

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

    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };

    // Create function that adds two integers
    let int_ty = ctx.ty_int();
    let fn_ty = ctx.ty_func(&[int_ty, int_ty], int_ty);
    let test_fn = ctx.declare_int_fn("test_add", fn_ty);

    unsafe {
        let builder = NonNull::new_unchecked(
            llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(ctx.llcx).as_ptr())
        );
        
        let bb = NonNull::new_unchecked(
            llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(ctx.llcx).as_ptr(),
                NonNull::from(test_fn).as_ptr(),
                UNNAMED,
            )
        );

        llvm_sys::core::LLVMPositionBuilderAtEnd(builder.as_ptr(), bb.as_ptr());

        let param0 = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 0);
        let param1 = llvm_sys::core::LLVMGetParam(NonNull::from(test_fn).as_ptr(), 1);
        
        let sum = llvm_sys::core::LLVMBuildAdd(
            builder.as_ptr(),
            param0,
            param1,
            UNNAMED,
        );
        
        llvm_sys::core::LLVMBuildRet(builder.as_ptr(), sum);
        llvm_sys::core::LLVMDisposeBuilder(builder.as_ptr());
    }

    // Verify the function
    assert!(module.verify().is_none(), "Function verification failed");

    // Check the generated IR
    let module_str = module.to_str().to_string();
    assert!(module_str.contains("define internal i32 @test_add(i32, i32)"));
    assert!(module_str.contains("add i32"));
    assert!(module_str.contains("ret i32"));
}

#[test]
fn test_optimization_constant_folding() {
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

    let mut literals = Rodeo::new();
    let ctx = unsafe { CodegenCx::new(&literals, &module, &target) };
    
    // Create function that returns a constant expression
    let fn_ty = ctx.ty_func(&[], ctx.ty_int());
    let test_fn = ctx.declare_int_fn("test_const_fold", fn_ty);
    
    unsafe {
        let builder = NonNull::new_unchecked(
            llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(ctx.llcx).as_ptr())
        );
        
        let bb = NonNull::new_unchecked(
            llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(ctx.llcx).as_ptr(),
                NonNull::from(test_fn).as_ptr(),
                UNNAMED,
            )
        );

        llvm_sys::core::LLVMPositionBuilderAtEnd(builder.as_ptr(), bb.as_ptr());
        
        // Build: return 2 + 3 * 4
        let two = ctx.const_int(2);
        let three = ctx.const_int(3);
        let four = ctx.const_int(4);
        
        let mul = llvm_sys::core::LLVMBuildMul(
            builder.as_ptr(),
            NonNull::from(three).as_ptr(),
            NonNull::from(four).as_ptr(),
            UNNAMED,
        );
        
        let sum = llvm_sys::core::LLVMBuildAdd(
            builder.as_ptr(),
            NonNull::from(two).as_ptr(),
            mul,
            UNNAMED,
        );
        
        llvm_sys::core::LLVMBuildRet(builder.as_ptr(), sum);
        llvm_sys::core::LLVMDisposeBuilder(builder.as_ptr());
    }

    // Get IR before optimization
    let before_opt = module.to_str().to_string();
    assert!(before_opt.contains("mul"));
    assert!(before_opt.contains("add"));

    // Run optimization
    module.optimize();
    assert!(module.verify().is_none(), "Optimization produced invalid module");

    // Get IR after optimization
    let after_opt = module.to_str().to_string();
    
    // The expression should be constant folded to return 14
    assert!(after_opt.contains("ret i32 14"));
}
