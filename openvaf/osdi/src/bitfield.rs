use core::ptr::NonNull;
use std::mem::size_of;

use llvm_sys::core::{
    LLVMBuildAnd, LLVMBuildGEP2, LLVMBuildICmp, LLVMBuildLoad2, LLVMBuildOr, LLVMBuildStore,
};
use llvm_sys::LLVMIntPredicate::{LLVMIntEQ, LLVMIntNE};
use mir_llvm::{CodegenCx, MemLoc, UNNAMED};
type Word = u32;
const WORD_BYTES: u32 = size_of::<Word>() as u32;
const WORD_BITS: u32 = WORD_BYTES * 8;

fn word_index_and_mask(pos: u32) -> (u32, u32) {
    let word_index = pos / WORD_BITS;
    let mask = 1 << (pos % WORD_BITS);
    (word_index, mask)
}

fn word_cnt(len: u32) -> u32 {
    (len + WORD_BITS - 1) / WORD_BITS
}

pub fn arr_ty<'ll>(len: u32, cx: &CodegenCx<'_, 'll>) -> &'ll llvm_sys::LLVMType {
    cx.ty_array(cx.ty_int(), word_cnt(len))
}
pub unsafe fn word_ptr_and_mask<'ll>(
    cx: &CodegenCx<'_, 'll>,
    pos: u32,
    arr_ptr: &'ll llvm_sys::LLVMValue,
    arr_ty: &'ll llvm_sys::LLVMType,
    llbuilder: &llvm_sys::LLVMBuilder,
) -> (&'ll llvm_sys::LLVMValue, &'ll llvm_sys::LLVMValue) {
    let (idx, mask) = word_index_and_mask(pos);
    let zero = cx.const_int(0);
    let pos = cx.const_unsigned_int(idx);

    // Create an array of pointers without casting
    let indices = [zero, pos];

    let word_ptr = LLVMBuildGEP2(
        NonNull::from(llbuilder).as_ptr(),
        NonNull::from(arr_ty).as_ptr(),
        NonNull::from(arr_ptr).as_ptr(),
        indices.as_ptr() as *mut *mut _,
        2,
        UNNAMED,
    );

    let mask = cx.const_unsigned_int(mask);
    // Convert the raw pointer back to a reference
    (unsafe { &*word_ptr }, mask)
}

pub unsafe fn is_set<'ll>(
    cx: &CodegenCx<'_, 'll>,
    pos: u32,
    arr_ptr: &'ll llvm_sys::LLVMValue,
    arr_ty: &'ll llvm_sys::LLVMType,
    llbuilder: &llvm_sys::LLVMBuilder,
) -> &'ll llvm_sys::LLVMValue {
    let (ptr, mask) = word_ptr_and_mask(cx, pos, arr_ptr, arr_ty, llbuilder);
    let word = LLVMBuildLoad2(
        NonNull::from(llbuilder).as_ptr(),
        NonNull::from(cx.ty_int()).as_ptr(),
        NonNull::from(ptr).as_ptr(),
        UNNAMED,
    );
    let is_set = LLVMBuildAnd(
        NonNull::from(llbuilder).as_ptr(),
        word,
        NonNull::from(mask).as_ptr(),
        UNNAMED,
    );
    let zero = cx.const_int(0);
    &*LLVMBuildICmp(
        NonNull::from(llbuilder).as_ptr(),
        LLVMIntNE,
        is_set,
        NonNull::from(zero).as_ptr(),
        UNNAMED,
    )
}

pub unsafe fn set_bit<'ll>(
    cx: &CodegenCx<'_, 'll>,
    pos: u32,
    arr_ptr: &'ll llvm_sys::LLVMValue,
    arr_ty: &'ll llvm_sys::LLVMType,
    llbuilder: &llvm_sys::LLVMBuilder,
) {
    let (ptr, mask) = word_ptr_and_mask(cx, pos, arr_ptr, arr_ty, llbuilder);
    let mut word = LLVMBuildLoad2(
        NonNull::from(llbuilder).as_ptr(),
        NonNull::from(cx.ty_int()).as_ptr(),
        NonNull::from(ptr).as_ptr(),
        UNNAMED,
    );
    word =
        LLVMBuildOr(NonNull::from(llbuilder).as_ptr(), word, NonNull::from(mask).as_ptr(), UNNAMED);
    LLVMBuildStore(NonNull::from(llbuilder).as_ptr(), word, NonNull::from(ptr).as_ptr());
}

pub unsafe fn is_flag_set_mem<'ll>(
    cx: &CodegenCx<'_, 'll>,
    flag: u32,
    val: &MemLoc<'ll>,
    llbuilder: &llvm_sys::LLVMBuilder,
) -> &'ll llvm_sys::LLVMValue {
    is_flag_set(cx, flag, val.read(llbuilder), llbuilder)
}

// pub unsafe fn is_flag_unset_mem<'ll>(
//     cx: &CodegenCx<'_, 'll>,
//     flag: u32,
//     val: MemLoc<'ll>,
//     llbuilder: &llvm_sys::LLVMBuilder,
// ) -> &'ll llvm_sys::LLVMValue {
//     is_flag_unset(cx, flag, val.read(llbuilder), llbuilder)
// }

pub unsafe fn is_flag_set<'ll>(
    cx: &CodegenCx<'_, 'll>,
    flag: u32,
    val: &'ll llvm_sys::LLVMValue,
    llbuilder: &llvm_sys::LLVMBuilder,
) -> &'ll llvm_sys::LLVMValue {
    let mask = cx.const_unsigned_int(flag);
    let bits = LLVMBuildAnd(
        NonNull::from(llbuilder).as_ptr(),
        NonNull::from(mask).as_ptr(),
        NonNull::from(val).as_ptr(),
        UNNAMED,
    );
    unsafe {
        &*LLVMBuildICmp(
            NonNull::from(llbuilder).as_ptr(),
            LLVMIntNE,
            bits,
            NonNull::from(cx.const_int(0)).as_ptr(),
            UNNAMED,
        )
    }
}

pub unsafe fn is_flag_unset<'ll>(
    cx: &CodegenCx<'_, 'll>,
    flag: u32,
    val: &'ll llvm_sys::LLVMValue,
    llbuilder: &llvm_sys::LLVMBuilder,
) -> &'ll llvm_sys::LLVMValue {
    let mask = cx.const_unsigned_int(flag);
    let bits = LLVMBuildAnd(
        NonNull::from(llbuilder).as_ptr(),
        NonNull::from(mask).as_ptr(),
        NonNull::from(val).as_ptr(),
        UNNAMED,
    );
    unsafe {
        &*LLVMBuildICmp(
            NonNull::from(llbuilder).as_ptr(),
            LLVMIntEQ,
            bits,
            NonNull::from(cx.const_int(0)).as_ptr(),
            UNNAMED,
        )
    }
}
