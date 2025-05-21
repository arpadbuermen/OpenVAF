use libc::c_char;
use std::ffi::CString;

/**
 * Extra stuff not in llvm-sys
 */
use crate::{Attribute, Context, Value, Type};

unsafe extern "C" {

    pub unsafe fn LLVMMDNodeInContext<'a>(
        C: &'a Context,
        Vals: *mut Value,
        Count: ::libc::c_uint,
    ) -> &'a Value;

    pub unsafe fn LLVMGetMDKindID<'a>(
        Name: *const ::libc::c_char, 
        SLen: ::libc::c_uint
    ) -> ::libc::c_uint;

    pub unsafe fn LLVMSetMetadata<'a>(
        Val: &'a Value, 
        KindID: ::libc::c_uint, 
        Node: &'a Value
    );

   pub fn LLVMAddDereferenceableAttr<'a>(
        V: &'a Value,
        Idx: u32,
        Bytes: u64
    );

    pub unsafe fn LLVMSizeOfType<'a>(Ty: & Type) -> u64;
}

pub unsafe fn mark_load_readonly<'a>(load_inst: &'a Value, llcx: &'a Context) {
    let empty_node = LLVMMDNodeInContext(llcx, std::ptr::null_mut(), 0);
    let c_string = CString::new("invariant.load").expect("CString::new failed");
    let len: u32 = c_string.as_bytes().len() as u32;
    let kind_id = LLVMGetMDKindID(c_string.into_raw(), len);
    LLVMSetMetadata(load_inst, kind_id, empty_node);
}
