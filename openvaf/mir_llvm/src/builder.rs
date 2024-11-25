use std::slice;

use arrayvec::ArrayVec;
use libc::c_uint;
use llvm_sys::core::LLVMBuildExtractValue;
use mir::{
    Block, ControlFlowGraph, FuncRef, Function, Inst, Opcode, Param, PhiNode, Value, ValueDef,
    F_ZERO, ZERO,
};
use typed_index_collections::TiVec;

use crate::callbacks::CallbackFun;
use crate::{CodegenCx, UNNAMED};

#[derive(Clone)]
pub struct MemLoc<'ll> {
    pub ptr: &'ll llvm_sys::LLVMValue,
    pub ptr_ty: &'ll llvm_sys::LLVMType,
    pub ty: &'ll llvm_sys::LLVMType,
    pub indices: Box<[&'ll llvm_sys::LLVMValue]>,
}

impl<'ll> MemLoc<'ll> {
    pub fn struct_gep(
        ptr: &'ll llvm_sys::LLVMValue,
        ptr_ty: &'ll llvm_sys::LLVMType,
        ty: &'ll llvm_sys::LLVMType,
        idx: u32,
        cx: &CodegenCx<'_, 'll>,
    ) -> MemLoc<'ll> {
        MemLoc {
            ptr,
            ptr_ty,
            ty,
            indices: vec![cx.const_unsigned_int(0), cx.const_unsigned_int(idx)].into_boxed_slice(),
        }
    }
    /// # Safety
    ///
    /// ptr_ty, ty and indices must be valid for ptr
    pub unsafe fn read(&self, llbuilder: &llvm_sys::LLVMBuilder) -> &'ll llvm_sys::LLVMValue {
        // Convert references to raw pointers

        // Call read_with_ptr and convert the result back to a reference
        &*(self.read_with_ptr(NonNull::from(llbuilder).as_ptr(), NonNull::from(self.ptr).as_ptr()))
    }

    /// # Safety
    ///
    /// ptr_ty, ty and indices must be valid for ptr
    pub unsafe fn read_with_ptr(
        &self,
        llbuilder: *mut llvm_sys::LLVMBuilder,
        ptr: *mut llvm_sys::LLVMValue,
    ) -> *mut llvm_sys::LLVMValue {
        let ptr = self.to_ptr_from(llbuilder, ptr);
        // SAFETY: We're calling an unsafe LLVM function and trusting that it returns a valid value
        unsafe {
            llvm_sys::core::LLVMBuildLoad2(llbuilder, self.ty as *const _ as *mut _, ptr, UNNAMED)
        }
    }

    /// # Safety
    ///
    /// ptr_ty and indices must be valid for ptr
    pub unsafe fn to_ptr(&self, llbuilder: *mut llvm_sys::LLVMBuilder) -> *mut llvm_sys::LLVMValue {
        self.to_ptr_from(llbuilder, self.ptr as *const _ as *mut _)
    }

    /// # Safety
    ///
    /// ptr_ty and indices must be valid for ptr
    pub unsafe fn to_ptr_from(
        &self,
        llbuilder: *mut llvm_sys::LLVMBuilder,
        mut ptr: *mut llvm_sys::LLVMValue,
    ) -> *mut llvm_sys::LLVMValue {
        if !self.indices.is_empty() {
            ptr = llvm_sys::core::LLVMBuildGEP2(
                llbuilder,
                self.ptr_ty as *const _ as *mut _,
                ptr,
                self.indices.as_ptr() as *mut *mut _,
                self.indices.len() as u32,
                UNNAMED,
            );
        }
        ptr
    }
}

impl<'ll> From<MemLoc<'ll>> for BuilderVal<'ll> {
    fn from(loc: MemLoc<'ll>) -> Self {
        BuilderVal::Load(Box::new(loc))
    }
}

#[derive(Clone)]
pub enum BuilderVal<'ll> {
    Undef,
    Eager(&'ll llvm_sys::LLVMValue),
    Load(Box<MemLoc<'ll>>),
    Call(Box<CallbackFun<'ll>>),
}

impl<'ll> From<&'ll llvm_sys::LLVMValue> for BuilderVal<'ll> {
    fn from(val: &'ll llvm_sys::LLVMValue) -> Self {
        BuilderVal::Eager(val)
    }
}

impl<'ll> BuilderVal<'ll> {
    /// # Safety
    ///
    /// For Self::Load and Self::Call, the values must be valid
    pub unsafe fn get(&self, builder: &Builder<'_, '_, 'll>) -> &'ll llvm_sys::LLVMValue {
        match self {
            BuilderVal::Undef => unreachable!("attempted to read undefined value"),
            BuilderVal::Eager(val) => val,
            BuilderVal::Load(loc) => loc.read(builder.llbuilder),
            BuilderVal::Call(call) => builder.call(call.fun_ty, call.fun, &call.state),
        }
    }

    /// # Safety
    ///
    /// For Self::Load and Self::Call, the values must be valid
    pub unsafe fn get_ty(&self, builder: &Builder<'_, '_, 'll>) -> Option<&'ll llvm_sys::LLVMType> {
        let ty = match self {
            BuilderVal::Undef => return None,
            BuilderVal::Eager(val) => builder.cx.val_ty(val),
            BuilderVal::Load(loc) => loc.ty,
            BuilderVal::Call(call) => {
                // SAFETY: We're converting a reference to a raw pointer, then calling an LLVM function,
                // and finally converting the result back to a reference. This is safe as long as LLVM
                // behaves correctly and the input type is valid.
                unsafe {
                    let fun_ty_ptr = call.fun_ty as *const _ as llvm_sys::prelude::LLVMTypeRef;
                    let return_ty_ptr = llvm_sys::core::LLVMGetReturnType(fun_ty_ptr);
                    &*return_ty_ptr
                }
            }
        };
        Some(ty)
    }
}

// All Builders must have an llfn associated with them
#[must_use]
pub struct Builder<'a, 'cx, 'll> {
    pub llbuilder: &'a mut llvm_sys::LLVMBuilder,
    pub cx: &'a CodegenCx<'cx, 'll>,
    pub func: &'a Function,
    pub blocks: TiVec<Block, Option<&'ll llvm_sys::LLVMBasicBlock>>,
    pub values: TiVec<Value, BuilderVal<'ll>>,
    pub params: TiVec<Param, BuilderVal<'ll>>,
    pub callbacks: TiVec<FuncRef, Option<CallbackFun<'ll>>>,
    pub prepend_pos: &'ll llvm_sys::LLVMBasicBlock,
    pub unfinished_phis: Vec<(PhiNode, &'ll llvm_sys::LLVMValue)>,
    pub fun: &'ll llvm_sys::LLVMValue,
}

impl Drop for Builder<'_, '_, '_> {
    fn drop(&mut self) {
        unsafe {
            llvm_sys::core::LLVMDisposeBuilder(&mut *(self.llbuilder as *mut _));
        }
    }
}

pub enum FastMathMode {
    Full,
    Partial,
    Disabled,
}
impl<'a, 'cx, 'll> Builder<'a, 'cx, 'll> {
    pub fn new(
        cx: &'a CodegenCx<'cx, 'll>,
        mir_func: &'a Function,
        llfunc: &'ll llvm_sys::LLVMValue,
    ) -> Self {
        let entry = unsafe {
            llvm_sys::core::LLVMAppendBasicBlockInContext(
                NonNull::from(cx.llcx).as_ptr(),
                NonNull::from(llfunc).as_ptr(),
                UNNAMED,
            )
        };
        let llbuilder =
            unsafe { llvm_sys::core::LLVMCreateBuilderInContext(cx.llcx as *const _ as *mut _) };
        let mut blocks: TiVec<_, _> = vec![None; mir_func.layout.num_blocks()].into();
        for bb in mir_func.layout.blocks() {
            blocks[bb] = unsafe {
                Some(llvm_sys::core::LLVMAppendBasicBlockInContext(
                    NonNull::from(cx.llcx).as_ptr(),
                    NonNull::from(llfunc).as_ptr(),
                    UNNAMED,
                ) as *mut _)
            };
        }
        unsafe { llvm_sys::core::LLVMPositionBuilderAtEnd(llbuilder, entry) };

        Builder {
            llbuilder: unsafe { &mut *llbuilder },
            cx,
            func: mir_func,
            blocks: blocks.into_iter().map(|b| b.map(|ptr| unsafe { &*ptr })).collect(),
            values: vec![BuilderVal::Undef; mir_func.dfg.num_values()].into(),
            params: Default::default(),
            callbacks: Default::default(),
            fun: llfunc,
            prepend_pos: unsafe { &*entry },
            unfinished_phis: Vec::new(),
        }
    }
}

use std::ptr::NonNull;

impl<'ll> Builder<'_, '_, 'll> {
    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    /// Must be called in the entry block of the function
    pub unsafe fn alloca(&self, ty: &'ll llvm_sys::LLVMType) -> &'ll llvm_sys::LLVMValue {
        &*(llvm_sys::core::LLVMBuildAlloca(
            self.llbuilder as *const _ as *mut _,
            ty as *const _ as *mut _,
            UNNAMED,
        ) as *const _)
    }

    /// # Safety
    /// Only correct llvm api calls must be performed within build_then and build_else
    /// Their return types must match and cond must be a bool
    pub unsafe fn add_branching_select(
        &mut self,
        cond: &'ll llvm_sys::LLVMValue,
        build_then: impl FnOnce(&mut Self) -> &'ll llvm_sys::LLVMValue,
        build_else: impl FnOnce(&mut Self) -> &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        let start = self.prepend_pos;
        let exit = llvm_sys::core::LLVMAppendBasicBlockInContext(
            NonNull::from(self.cx.llcx).as_ptr(),
            NonNull::from(self.fun).as_ptr(),
            UNNAMED,
        );
        let then_bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
            NonNull::from(self.cx.llcx).as_ptr(),
            NonNull::from(self.fun).as_ptr(),
            UNNAMED,
        );
        llvm_sys::core::LLVMPositionBuilderAtEnd(self.llbuilder, then_bb);
        self.prepend_pos = &*(then_bb as *const _);
        let then_val = build_then(self);
        llvm_sys::core::LLVMBuildBr(self.llbuilder, exit);

        let else_bb = llvm_sys::core::LLVMAppendBasicBlockInContext(
            NonNull::from(self.cx.llcx).as_ptr(),
            NonNull::from(self.fun).as_ptr(),
            UNNAMED,
        );
        llvm_sys::core::LLVMPositionBuilderAtEnd(self.llbuilder, else_bb);
        self.prepend_pos = &*(else_bb as *const _);
        let else_val = build_else(self);
        llvm_sys::core::LLVMBuildBr(self.llbuilder, exit);

        llvm_sys::core::LLVMPositionBuilderAtEnd(self.llbuilder, NonNull::from(start).as_ptr());
        llvm_sys::core::LLVMBuildCondBr(
            self.llbuilder,
            NonNull::from(cond).as_ptr(),
            then_bb,
            else_bb,
        );

        self.prepend_pos = &*(exit as *const _);
        llvm_sys::core::LLVMPositionBuilderAtEnd(self.llbuilder, exit);
        let phi = llvm_sys::core::LLVMBuildPhi(
            self.llbuilder,
            llvm_sys::core::LLVMTypeOf(NonNull::from(then_val).as_ptr()),
            UNNAMED,
        );
        let mut incoming_blocks = [then_bb, else_bb];
        llvm_sys::core::LLVMAddIncoming(
            phi,
            [NonNull::from(then_val).as_ptr(), NonNull::from(else_val).as_ptr()].as_ptr() as *mut _,
            incoming_blocks.as_mut_ptr(),
            2,
        );
        &*phi
    }

    /// # Safety
    /// Only correct llvm api calls must be performed within build_then and build_else
    /// Their return types must match and cond must be a bool
    pub unsafe fn select(
        &mut self,
        cond: &'ll llvm_sys::LLVMValue,
        then_val: &'ll llvm_sys::LLVMValue,
        else_val: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        let result = llvm_sys::core::LLVMBuildSelect(
            self.llbuilder,
            NonNull::from(cond).as_ptr(),
            NonNull::from(then_val).as_ptr(),
            NonNull::from(else_val).as_ptr(),
            UNNAMED,
        );
        &*(result as *const _)
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn typed_gep(
        &mut self,
        arr_ty: &'ll llvm_sys::LLVMType,
        ptr: &'ll llvm_sys::LLVMValue,
        indices: &[&'ll llvm_sys::LLVMValue],
    ) -> &'ll llvm_sys::LLVMValue {
        let result = llvm_sys::core::LLVMBuildGEP2(
            self.llbuilder,
            NonNull::from(arr_ty).as_ptr(),
            NonNull::from(ptr).as_ptr(),
            indices.as_ptr() as *const _ as *mut _,
            indices.len() as u32,
            UNNAMED,
        );

        // Safety: We're assuming that the LLVM API returns a valid pointer.
        // The lifetime 'll is tied to the Builder, which owns the LLVM context.
        NonNull::new(result).map(|nn| nn.as_ref()).expect("LLVM returned null pointer")
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn gep(
        &mut self,
        elem_ty: &'ll llvm_sys::LLVMType,
        ptr: &'ll llvm_sys::LLVMValue,
        indices: &[&'ll llvm_sys::LLVMValue],
    ) -> &'ll llvm_sys::LLVMValue {
        self.typed_gep(elem_ty, ptr, indices)
    }

    /// # Safety
    /// * Must not be called when a block that already contains a terminator is selected
    /// * struct_ty must be a valid struct type for this pointer and idx must be in bounds
    pub unsafe fn struct_gep(
        &mut self,
        struct_ty: &'ll llvm_sys::LLVMType,
        ptr: &'ll llvm_sys::LLVMValue,
        idx: u32,
    ) -> &'ll llvm_sys::LLVMValue {
        let result = llvm_sys::core::LLVMBuildStructGEP2(
            self.llbuilder,
            NonNull::from(struct_ty).as_ptr(),
            NonNull::from(ptr).as_ptr(),
            idx,
            UNNAMED,
        );

        // Safety: We're assuming that the LLVM API returns a valid pointer.
        // The lifetime 'll is tied to the Builder, which owns the LLVM context.
        NonNull::new(result).map(|nn| nn.as_ref()).expect("LLVM returned null pointer")
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn fat_ptr_get_ptr(
        &mut self,
        ptr: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        self.struct_gep(self.cx.ty_fat_ptr(), ptr, 0)
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn fat_ptr_get_meta(
        &mut self,
        ptr: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        self.struct_gep(self.cx.ty_fat_ptr(), ptr, 1)
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn fat_ptr_to_parts(
        &mut self,
        ptr: &'ll llvm_sys::LLVMValue,
    ) -> (&'ll llvm_sys::LLVMValue, &'ll llvm_sys::LLVMValue) {
        (self.fat_ptr_get_ptr(ptr), self.fat_ptr_get_meta(ptr))
    }

    /// # Safety
    /// * Must not be called when a block that already contains a terminator is selected
    pub unsafe fn call(
        &self,
        fun_ty: &'ll llvm_sys::LLVMType,
        fun: &'ll llvm_sys::LLVMValue,
        operands: &[&'ll llvm_sys::LLVMValue],
    ) -> &'ll llvm_sys::LLVMValue {
        let res = llvm_sys::core::LLVMBuildCall2(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(fun_ty).as_ptr(),
            NonNull::from(fun).as_ptr(),
            operands.as_ptr() as *mut _,
            operands.len() as u32,
            UNNAMED,
        );

        // forgett this is a real footgun
        let cconv = llvm_sys::core::LLVMGetFunctionCallConv(NonNull::from(fun).as_ptr());
        llvm_sys::core::LLVMSetInstructionCallConv(res, cconv);
        &*(res as *const _)
    }

    pub fn build_consts(&mut self) {
        for val in self.func.dfg.values() {
            match self.func.dfg.value_def(val) {
                ValueDef::Result(_, _) | ValueDef::Invalid => (),
                ValueDef::Param(param) => self.values[val] = self.params[param].clone(),
                ValueDef::Const(const_val) => {
                    self.values[val] = self.cx.const_val(&const_val).into();
                }
            }
        }
    }

    /// # Safety
    ///
    /// Must not be called if any block already contain any non-phi instruction (eg must not be
    /// called twice)
    pub unsafe fn build_func(&mut self) {
        let entry = self.func.layout.entry_block().unwrap();
        llvm_sys::core::LLVMBuildBr(
            self.llbuilder,
            NonNull::from(self.blocks[entry].unwrap()).as_ptr(),
        );
        let mut cfg = ControlFlowGraph::new();
        cfg.compute(self.func);
        let po: Vec<_> = cfg.postorder(self.func).collect();
        drop(cfg);
        for bb in po.into_iter().rev() {
            self.build_bb(bb)
        }

        for (phi, llval) in self.unfinished_phis.iter() {
            let (blocks, vals): (Vec<_>, Vec<_>) = self
                .func
                .dfg
                .phi_edges(phi)
                .map(|(bb, val)| {
                    self.select_bb_before_terminator(bb);
                    (self.blocks[bb].unwrap(), self.values[val].get(self))
                })
                .unzip();

            let mut incoming_vals: Vec<*mut llvm_sys::LLVMValue> =
                vals.iter().map(|&v| NonNull::from(v).as_ptr()).collect();
            let mut incoming_blocks: Vec<*mut llvm_sys::LLVMBasicBlock> =
                blocks.iter().map(|&b| NonNull::from(b).as_ptr()).collect();

            llvm_sys::core::LLVMAddIncoming(
                NonNull::from(*llval).as_ptr(),
                incoming_vals.as_mut_ptr(),
                incoming_blocks.as_mut_ptr(),
                vals.len() as c_uint,
            );
        }

        self.unfinished_phis.clear();
    }
    pub fn select_bb(&self, bb: Block) {
        unsafe {
            llvm_sys::core::LLVMPositionBuilderAtEnd(
                self.llbuilder as *const _ as *mut _,
                NonNull::from(self.blocks[bb].unwrap()).as_ptr(),
            );
        }
    }

    pub fn select_bb_before_terminator(&self, bb: Block) {
        let bb = self.blocks[bb].unwrap();
        unsafe {
            let bb_ptr = NonNull::from(bb).as_ptr();
            let inst = llvm_sys::core::LLVMGetLastInstruction(bb_ptr);
            llvm_sys::core::LLVMPositionBuilder(self.llbuilder as *const _ as *mut _, bb_ptr, inst);
        }
    }
    /// # Safety
    ///
    /// Must not be called if any non phi instruction has already been build for `bb`
    /// The means it must not be called twice for the same bloc
    pub unsafe fn build_bb(&mut self, bb: Block) {
        self.select_bb(bb);

        for inst in self.func.layout.block_insts(bb) {
            let fast_math = self.func.srclocs.get(inst).map_or(false, |loc| loc.0 < 0);
            self.build_inst(
                inst,
                if fast_math { FastMathMode::Partial } else { FastMathMode::Disabled },
            )
        }
    }

    /// # Safety
    /// must not be called multiple times
    /// a terminator must not be build for the exit bb trough other means
    pub unsafe fn ret(&mut self, val: &'ll llvm_sys::LLVMValue) {
        llvm_sys::core::LLVMBuildRet(self.llbuilder, NonNull::from(val).as_ptr());
    }
    /// # Safety
    /// must not be called multiple times
    /// a terminator must not be build for the exit bb trough other means
    pub unsafe fn ret_void(&mut self) {
        llvm_sys::core::LLVMBuildRetVoid(self.llbuilder as *mut _);
    }

    /// # Safety
    /// Must only be called when after the builder has been positioned
    /// Not Phis may be constructed for the current block after this function has been called
    /// Must not be called when the builder has selected a block that already contains a terminator
    pub unsafe fn build_inst(&mut self, inst: Inst, fast_math_mode: FastMathMode) {
        let (opcode, args) = match self.func.dfg.insts[inst] {
            mir::InstructionData::Unary { opcode, ref arg } => (opcode, slice::from_ref(arg)),
            mir::InstructionData::Binary { opcode, ref args } => (opcode, args.as_slice()),
            mir::InstructionData::Branch { cond, then_dst, else_dst, .. } => {
                llvm_sys::core::LLVMBuildCondBr(
                    self.llbuilder as *const _ as *mut _,
                    NonNull::from(self.values[cond].get(self)).as_ptr(),
                    NonNull::from(self.blocks[then_dst].unwrap()).as_ptr(),
                    NonNull::from(self.blocks[else_dst].unwrap()).as_ptr(),
                );
                return;
            }
            mir::InstructionData::PhiNode(ref phi) => {
                // TODO does this always produce a valid value?
                let ty = self
                    .func
                    .dfg
                    .phi_edges(phi)
                    .find_map(|(_, val)| self.values[val].get_ty(self))
                    .unwrap();
                let llval = llvm_sys::core::LLVMBuildPhi(
                    self.llbuilder,
                    NonNull::from(ty).as_ptr(),
                    UNNAMED,
                );
                let llval_ref: &'ll llvm_sys::LLVMValue = &*(llval as *const _);
                self.unfinished_phis.push((phi.clone(), llval_ref));
                let res = self.func.dfg.first_result(inst);
                self.values[res] = BuilderVal::Eager(llval_ref);
                return;
            }
            mir::InstructionData::Jump { destination } => {
                llvm_sys::core::LLVMBuildBr(
                    self.llbuilder,
                    NonNull::from(self.blocks[destination].unwrap()).as_ptr(),
                );
                return;
            }
            mir::InstructionData::Call { func_ref, ref args } => {
                let callback = if let Some(res) = self.callbacks[func_ref].as_ref() {
                    res
                } else {
                    return; // assume nooop
                };

                let args = args.as_slice(&self.func.dfg.insts.value_lists);
                let args = args.iter().map(|operand| self.values[*operand].get(self));

                if callback.num_state != 0 {
                    let args: Vec<_> = args.collect();
                    let num_iter = callback.state.len() as u32 / callback.num_state;
                    for i in 0..num_iter {
                        let start = (i * callback.num_state) as usize;
                        let end = ((i + 1) * callback.num_state) as usize;
                        let operands: Vec<_> = callback.state[start..end]
                            .iter()
                            .copied()
                            .chain(args.iter().copied())
                            .collect();
                        self.call(callback.fun_ty, callback.fun, &operands);
                        debug_assert!(self.func.dfg.inst_results(inst).is_empty());
                    }
                } else {
                    let operands: Vec<_> = callback.state.iter().copied().chain(args).collect();
                    let res = self.call(callback.fun_ty, callback.fun, &operands);
                    let inst_res = self.func.dfg.inst_results(inst);

                    match inst_res {
                        [] => (),
                        [val] => self.values[*val] = res.into(),
                        vals => {
                            for (i, val) in vals.iter().enumerate() {
                                let res = LLVMBuildExtractValue(
                                    self.llbuilder,
                                    NonNull::from(res).as_ptr(),
                                    i as u32,
                                    UNNAMED,
                                );
                                let res_ref: &'ll llvm_sys::LLVMValue = &*(res as *const _);
                                self.values[*val] = BuilderVal::Eager(res_ref);
                            }
                        }
                    }
                }
                return;
            }
        };

        let val = match opcode {
            Opcode::Inot | Opcode::Bnot => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildNot(self.llbuilder, arg, UNNAMED)
            }

            Opcode::Ineg => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildNeg(self.llbuilder, arg, UNNAMED)
            }
            Opcode::Fneg => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFNeg(self.llbuilder, arg, UNNAMED)
            }
            Opcode::IFcast => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildSIToFP(
                    self.llbuilder,
                    arg,
                    NonNull::from(self.cx.ty_double()).as_ptr(),
                    UNNAMED,
                )
            }
            Opcode::BFcast => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildUIToFP(
                    self.llbuilder,
                    arg,
                    NonNull::from(self.cx.ty_double()).as_ptr(),
                    UNNAMED,
                )
            }
            Opcode::BIcast => {
                let arg = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildIntCast2(
                    self.llbuilder,
                    arg,
                    NonNull::from(self.cx.ty_int()).as_ptr(),
                    0,
                    UNNAMED,
                )
            }
            Opcode::IBcast => NonNull::from(
                self.build_int_cmp(&[args[0], ZERO], llvm_sys::LLVMIntPredicate::LLVMIntNE),
            )
            .as_ptr(),
            Opcode::FBcast => NonNull::from(
                self.build_real_cmp(&[args[0], F_ZERO], llvm_sys::LLVMRealPredicate::LLVMRealONE),
            )
            .as_ptr(),
            Opcode::Iadd => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildAdd(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Isub => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildSub(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Imul => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildMul(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Idiv => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildSDiv(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Irem => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildSRem(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Ishl => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildShl(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Ishr => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildLShr(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Ixor => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildXor(self.llbuilder, lhs, rhs, UNNAMED)
            }

            Opcode::Iand => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildAnd(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Ior => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildOr(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Fadd => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFAdd(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Fsub => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFSub(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Fmul => {
                if matches!(self.values[args[0]], BuilderVal::Undef) {
                    panic!(
                        "{} {}",
                        self.func.dfg.display_inst(inst),
                        self.func.layout.inst_block(inst).unwrap()
                    );
                }
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFMul(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Fdiv => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFDiv(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Frem => {
                let lhs = NonNull::from(self.values[args[0]].get(self)).as_ptr();
                let rhs = NonNull::from(self.values[args[1]].get(self)).as_ptr();
                llvm_sys::core::LLVMBuildFRem(self.llbuilder, lhs, rhs, UNNAMED)
            }
            Opcode::Ilt => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntSLT))
                    .as_ptr()
            }
            Opcode::Igt => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntSGT))
                    .as_ptr()
            }
            Opcode::Flt => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealOLT))
                    .as_ptr()
            }
            Opcode::Fgt => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealOGT))
                    .as_ptr()
            }
            Opcode::Ile => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntSLE))
                    .as_ptr()
            }
            Opcode::Ige => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntSGE))
                    .as_ptr()
            }
            Opcode::Fle => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealOLE))
                    .as_ptr()
            }
            Opcode::Fge => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealOGE))
                    .as_ptr()
            }
            Opcode::Ieq | Opcode::Beq => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntEQ))
                    .as_ptr()
            }
            Opcode::Feq => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealOEQ))
                    .as_ptr()
            }
            Opcode::Fne => {
                NonNull::from(self.build_real_cmp(args, llvm_sys::LLVMRealPredicate::LLVMRealONE))
                    .as_ptr()
            }
            Opcode::Bne | Opcode::Ine => {
                NonNull::from(self.build_int_cmp(args, llvm_sys::LLVMIntPredicate::LLVMIntNE))
                    .as_ptr()
            }
            Opcode::FIcast => NonNull::from(self.intrinsic(args, "llvm.lround.i32.f64")).as_ptr(),
            Opcode::Seq => NonNull::from(self.strcmp(args, false)).as_ptr(),
            Opcode::Sne => NonNull::from(self.strcmp(args, true)).as_ptr(),
            Opcode::Sqrt => NonNull::from(self.intrinsic(args, "llvm.sqrt.f64")).as_ptr(),
            Opcode::Exp => NonNull::from(self.intrinsic(args, "llvm.exp.f64")).as_ptr(),
            Opcode::Ln => NonNull::from(self.intrinsic(args, "llvm.log.f64")).as_ptr(),
            Opcode::Log => NonNull::from(self.intrinsic(args, "llvm.log10.f64")).as_ptr(),
            Opcode::Clog2 => {
                let leading_zeros =
                    NonNull::from(self.intrinsic(&[args[0], true.into()], "llvm.ctlz")).as_ptr();
                let total_bits = NonNull::from(self.cx.const_int(32)).as_ptr();
                llvm_sys::core::LLVMBuildSub(self.llbuilder, total_bits, leading_zeros, UNNAMED)
            }
            Opcode::Floor => NonNull::from(self.intrinsic(args, "llvm.floor.f64")).as_ptr(),
            Opcode::Ceil => NonNull::from(self.intrinsic(args, "llvm.ceil.f64")).as_ptr(),
            Opcode::Sin => NonNull::from(self.intrinsic(args, "llvm.sin.f64")).as_ptr(),
            Opcode::Cos => NonNull::from(self.intrinsic(args, "llvm.cos.f64")).as_ptr(),
            Opcode::Tan => NonNull::from(self.intrinsic(args, "tan")).as_ptr(),
            Opcode::Hypot => NonNull::from(self.intrinsic(args, "hypot")).as_ptr(),
            Opcode::Asin => NonNull::from(self.intrinsic(args, "asin")).as_ptr(),
            Opcode::Acos => NonNull::from(self.intrinsic(args, "acos")).as_ptr(),
            Opcode::Atan => NonNull::from(self.intrinsic(args, "atan")).as_ptr(),
            Opcode::Atan2 => NonNull::from(self.intrinsic(args, "atan2")).as_ptr(),
            Opcode::Sinh => NonNull::from(self.intrinsic(args, "sinh")).as_ptr(),
            Opcode::Cosh => NonNull::from(self.intrinsic(args, "cosh")).as_ptr(),
            Opcode::Tanh => NonNull::from(self.intrinsic(args, "tanh")).as_ptr(),
            Opcode::Asinh => NonNull::from(self.intrinsic(args, "asinh")).as_ptr(),
            Opcode::Acosh => NonNull::from(self.intrinsic(args, "acosh")).as_ptr(),
            Opcode::Atanh => NonNull::from(self.intrinsic(args, "atanh")).as_ptr(),
            Opcode::Pow => NonNull::from(self.intrinsic(args, "llvm.pow.f64")).as_ptr(),
            Opcode::OptBarrier => NonNull::from(self.values[args[0]].get(self)).as_ptr(),
            Opcode::Br | Opcode::Jmp | Opcode::Call | Opcode::Phi => unreachable!(),
        };

        let res = self.func.dfg.first_result(inst);
        let val_ref: &'ll llvm_sys::LLVMValue = &*val;
        self.values[res] = BuilderVal::Eager(val_ref);

        if matches!(
            opcode,
            Opcode::Fneg
                | Opcode::Feq
                | Opcode::Fne
                | Opcode::Fadd
                | Opcode::Fsub
                | Opcode::Fmul
                | Opcode::Fdiv
                | Opcode::Frem
                | Opcode::Flt
                | Opcode::Fgt
                | Opcode::Fle
                | Opcode::Fge
                | Opcode::Sqrt
                | Opcode::Exp
                | Opcode::Ln
                | Opcode::Log
                | Opcode::Clog2
                | Opcode::Floor
                | Opcode::Ceil
                | Opcode::Sin
                | Opcode::Cos
                | Opcode::Tan
                | Opcode::Hypot
                | Opcode::Asin
                | Opcode::Acos
                | Opcode::Atan
                | Opcode::Atan2
                | Opcode::Sinh
                | Opcode::Cosh
                | Opcode::Tanh
                | Opcode::Asinh
                | Opcode::Acosh
                | Opcode::Atanh
                | Opcode::Pow
        ) {
            match fast_math_mode {
                FastMathMode::Full => {
                    // Use all fast-math flags
                    let fast_math_flags: c_uint = 0x1F; // This represents all flags set
                    unsafe {
                        llvm_sys::core::LLVMSetFastMathFlags(val, fast_math_flags);
                    }
                }
                FastMathMode::Partial => {
                    // Set specific fast-math flags
                    let fast_math_flags: c_uint = 0x01 | 0x02 | 0x10; // Reassoc | Reciprocal | Contract
                    unsafe {
                        llvm_sys::core::LLVMSetFastMathFlags(val, fast_math_flags);
                    }
                }
                FastMathMode::Disabled => (), // No fast-math flags
            }
        }
    }

    unsafe fn strcmp(&mut self, args: &[Value], invert: bool) -> &'ll llvm_sys::LLVMValue {
        let res = self.intrinsic(args, "strcmp");
        let predicate = if invert {
            llvm_sys::LLVMIntPredicate::LLVMIntNE
        } else {
            llvm_sys::LLVMIntPredicate::LLVMIntEQ
        };

        NonNull::new(llvm_sys::core::LLVMBuildICmp(
            self.llbuilder,
            predicate,
            NonNull::from(res).as_ptr(),
            NonNull::from(self.cx.const_int(0)).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn store(&self, ptr: &'ll llvm_sys::LLVMValue, val: &'ll llvm_sys::LLVMValue) {
        llvm_sys::core::LLVMBuildStore(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(val).as_ptr(),
            NonNull::from(ptr).as_ptr(),
        );
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn load(
        &self,
        ty: &'ll llvm_sys::LLVMType,
        ptr: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildLoad2(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(ty).as_ptr(),
            NonNull::from(ptr).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }
    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn imul(
        &self,
        val1: &'ll llvm_sys::LLVMValue,
        val2: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildMul(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(val1).as_ptr(),
            NonNull::from(val2).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn iadd(
        &self,
        val1: &'ll llvm_sys::LLVMValue,
        val2: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildAdd(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(val1).as_ptr(),
            NonNull::from(val2).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn ptr_diff(
        &self,
        ty: &'ll llvm_sys::LLVMType,
        ptr1: &'ll llvm_sys::LLVMValue,
        ptr2: &'ll llvm_sys::LLVMValue,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildPtrDiff2(
            self.llbuilder as *const _ as *mut _,
            NonNull::from(ty).as_ptr(),
            NonNull::from(ptr1).as_ptr(),
            NonNull::from(ptr2).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    ///
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn is_null_ptr(&self, ptr: &'ll llvm_sys::LLVMValue) -> &'ll llvm_sys::LLVMValue {
        let null_ptr = self.cx.const_null_ptr();
        NonNull::new(llvm_sys::core::LLVMBuildICmp(
            self.llbuilder as *const _ as *mut _,
            llvm_sys::LLVMIntPredicate::LLVMIntEQ,
            NonNull::from(null_ptr).as_ptr(),
            NonNull::from(ptr).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    unsafe fn build_int_cmp(
        &mut self,
        args: &[Value],
        predicate: llvm_sys::LLVMIntPredicate,
    ) -> &'ll llvm_sys::LLVMValue {
        let lhs = self.values[args[0]].get(self);
        let rhs = self.values[args[1]].get(self);
        self.int_cmp(lhs, rhs, predicate)
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn int_cmp(
        &self,
        lhs: &'ll llvm_sys::LLVMValue,
        rhs: &'ll llvm_sys::LLVMValue,
        predicate: llvm_sys::LLVMIntPredicate,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildICmp(
            self.llbuilder as *const _ as *mut _,
            predicate,
            NonNull::from(lhs).as_ptr(),
            NonNull::from(rhs).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    unsafe fn build_real_cmp(
        &mut self,
        args: &[Value],
        predicate: llvm_sys::LLVMRealPredicate,
    ) -> &'ll llvm_sys::LLVMValue {
        let lhs = self.values[args[0]].get(self);
        let rhs = self.values[args[1]].get(self);
        self.real_cmp(lhs, rhs, predicate)
    }

    /// # Safety
    /// Must not be called when a block that already contains a terminator is selected
    pub unsafe fn real_cmp(
        &mut self,
        lhs: &'ll llvm_sys::LLVMValue,
        rhs: &'ll llvm_sys::LLVMValue,
        predicate: llvm_sys::LLVMRealPredicate,
    ) -> &'ll llvm_sys::LLVMValue {
        NonNull::new(llvm_sys::core::LLVMBuildFCmp(
            self.llbuilder,
            predicate,
            NonNull::from(lhs).as_ptr(),
            NonNull::from(rhs).as_ptr(),
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }
    unsafe fn intrinsic(&mut self, args: &[Value], name: &'static str) -> &'ll llvm_sys::LLVMValue {
        let (ty, fun) =
            self.cx.intrinsic(name).unwrap_or_else(|| unreachable!("intrinsic {} not found", name));
        let args: ArrayVec<_, 2> = args.iter().map(|arg| self.values[*arg].get(self)).collect();

        NonNull::new(llvm_sys::core::LLVMBuildCall2(
            self.llbuilder,
            NonNull::from(ty).as_ptr(),
            NonNull::from(fun).as_ptr(),
            args.as_ptr() as *mut _,
            args.len() as u32,
            UNNAMED,
        ))
        .unwrap()
        .as_ref()
    }
}
