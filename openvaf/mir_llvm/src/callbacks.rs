use crate::{CodegenCx, UNNAMED};

#[derive(Clone)]
pub struct CallbackFun<'ll> {
    pub fun_ty: &'ll llvm_sys::LLVMType,
    pub fun: &'ll llvm_sys::LLVMValue,
    /// Some Callbacks need to read/modify some state (typically passed as pointers)
    /// outside of the arguments provided in Verilog-A.
    /// These arguments are always passed before any arguments specified in the CFG
    pub state: Box<[&'ll llvm_sys::LLVMValue]>,
    pub num_state: u32,
}

use core::ptr::NonNull;
impl<'ll> CodegenCx<'_, 'll> {
    pub fn const_callback(
        &self,
        args: &[&'ll llvm_sys::LLVMType],
        val: &'ll llvm_sys::LLVMValue,
    ) -> CallbackFun<'ll> {
        let name = self.local_callback_name();
        // println!("Creating const callback function: {}", name); // Logging the creation of the callback
        let fun_ty = self.ty_func(args, self.val_ty(val));
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(self.llcx).as_ptr(),
                NonNull::from(fun).as_ptr(),
                UNNAMED,
            );
            let builder =
                llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(self.llcx).as_ptr());
            llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
            llvm_sys::core::LLVMBuildRet(builder, NonNull::from(val).as_ptr());
            llvm_sys::core::LLVMDisposeBuilder(builder);
        }

        CallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn trivial_callbacks(&self, args: &[&'ll llvm_sys::LLVMType]) -> CallbackFun<'ll> {
        let name = self.local_callback_name();
        //println!("Generating trivial callback with name: {}", name); // Log when trivial callback is generated.
        let fun_ty = self.ty_func(args, self.ty_void());
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(self.llcx).as_ptr(),
                NonNull::from(fun).as_ptr(),
                UNNAMED,
            );
            let builder =
                llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(self.llcx).as_ptr());
            llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
            llvm_sys::core::LLVMBuildRetVoid(builder);
            llvm_sys::core::LLVMDisposeBuilder(builder);
        }

        CallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn const_return(&self, args: &[&'ll llvm_sys::LLVMType], idx: usize) -> CallbackFun<'ll> {
        let name = self.local_callback_name();
        let fun_ty = self.ty_func(args, args[idx]);
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(self.llcx).as_ptr(),
                NonNull::from(fun).as_ptr(),
                UNNAMED,
            );
            let builder =
                llvm_sys::core::LLVMCreateBuilderInContext(NonNull::from(self.llcx).as_ptr());
            llvm_sys::core::LLVMPositionBuilderAtEnd(builder, bb);
            let val = llvm_sys::core::LLVMGetParam(NonNull::from(fun).as_ptr(), idx as u32);
            llvm_sys::core::LLVMBuildRet(builder, val);
            llvm_sys::core::LLVMDisposeBuilder(builder);
        }
        CallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn local_callback_name(&self) -> String {
        let name = self.generate_local_symbol_name("cb");
        name
    }
}
