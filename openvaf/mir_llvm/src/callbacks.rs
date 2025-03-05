use llvm::UNNAMED;

use crate::CodegenCx;
use crate::builder::Builder;

pub trait InlineCallbackBuilder<'ll> {
    fn build_inline(&self, builder: &Builder<'_, '_, 'll>, state: &Box<[&'ll llvm::Value]>) -> &'ll llvm::Value; 
    fn return_type(&self, builder: &Builder<'_, '_, 'll>, state: &Box<[&'ll llvm::Value]>) -> &'ll llvm::Type;
}

impl<'ll> Clone for Box<dyn InlineCallbackBuilder<'ll>> {
    fn clone(&self) -> Box<dyn InlineCallbackBuilder<'ll>> {
        panic!("Box<dyn Trait> does not support Clone. Use Arc instead!");
    }
}

#[derive(Clone)]
pub struct BuiltCallbackFun<'ll> {
    pub fun_ty: &'ll llvm::Type,
    pub fun: &'ll llvm::Value,
    /// Some Callbacks need to read/modify some state (typically passed as pointers)
    /// outside of the arguments provided in Verilog-A.
    /// These arguments are always passed before any arguments specified in the CFG
    pub state: Box<[&'ll llvm::Value]>,
    pub num_state: u32,
}

#[derive(Clone)]
pub enum CallbackFun<'ll> {
    Inline{
        builder: Box<dyn InlineCallbackBuilder<'ll>>, 
        state: Box<[&'ll llvm::Value]>
    }, 
    Prebuilt(BuiltCallbackFun<'ll>)
}

impl<'ll> CodegenCx<'_, 'll> {
    pub fn const_callback(
        &self,
        args: &[&'ll llvm::Type],
        val: &'ll llvm::Value,
    ) -> BuiltCallbackFun<'ll> {
        let name = self.local_callback_name();
        let fun_ty = self.ty_func(args, self.val_ty(val));
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm::LLVMAppendBasicBlockInContext(self.llcx, fun, UNNAMED);
            let builder = llvm::LLVMCreateBuilderInContext(self.llcx);
            llvm::LLVMPositionBuilderAtEnd(builder, bb);
            llvm::LLVMBuildRet(builder, val);
            llvm::LLVMDisposeBuilder(builder);
        }

        BuiltCallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn trivial_callbacks(&self, args: &[&'ll llvm::Type]) -> BuiltCallbackFun<'ll> {
        let name = self.local_callback_name();
        let fun_ty = self.ty_func(args, self.ty_void());
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm::LLVMAppendBasicBlockInContext(self.llcx, fun, UNNAMED);
            let builder = llvm::LLVMCreateBuilderInContext(self.llcx);
            llvm::LLVMPositionBuilderAtEnd(builder, bb);
            llvm::LLVMBuildRetVoid(builder);
            llvm::LLVMDisposeBuilder(builder);
        }

        BuiltCallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn const_return(&self, args: &[&'ll llvm::Type], idx: usize) -> BuiltCallbackFun<'ll> {
        let name = self.local_callback_name();
        let fun_ty = self.ty_func(args, args[idx]);
        let fun = self.declare_int_fn(&name, fun_ty);
        unsafe {
            let bb = llvm::LLVMAppendBasicBlockInContext(self.llcx, fun, UNNAMED);
            let builder = llvm::LLVMCreateBuilderInContext(self.llcx);
            llvm::LLVMPositionBuilderAtEnd(builder, bb);
            let val = llvm::LLVMGetParam(fun, idx as u32);
            llvm::LLVMBuildRet(builder, val);
            llvm::LLVMDisposeBuilder(builder);
        }
        BuiltCallbackFun { fun_ty, fun, state: Box::new([]), num_state: 0 }
    }

    pub fn local_callback_name(&self) -> String {
        self.generate_local_symbol_name("cb")
    }
}
