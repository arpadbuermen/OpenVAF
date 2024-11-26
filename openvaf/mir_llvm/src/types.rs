use std::ffi::CString;

use libc::c_uint;
use llvm_sys::core::LLVMInt8TypeInContext;
use llvm_sys::prelude::LLVMBool;
use llvm_sys::{LLVMType as Type, LLVMValue as Value};

const FALSE: LLVMBool = 0;
const TRUE: LLVMBool = 1;
use core::ptr::NonNull;

use mir::Const;

use crate::CodegenCx;

pub struct Types<'ll> {
    pub double: &'ll Type,
    pub char: &'ll Type,
    pub int: &'ll Type,
    pub size: &'ll Type,
    pub ptr: &'ll Type,
    pub fat_ptr: &'ll Type,
    pub bool: &'ll Type,
    pub void: &'ll Type,
    pub null_ptr_val: &'ll Value,
}

impl<'ll> Types<'ll> {
    pub fn new(llcx: &'ll llvm_sys::LLVMContext, pointer_width: u32) -> Types<'ll> {
        unsafe {
            let char = LLVMInt8TypeInContext(NonNull::from(llcx).as_ptr());
            // we are using opaque pointers, with old llvm version that plain
            // means always using char pointers, with newer llvm version the
            // type is ignored anyway
            //let ptr = llvm_sys::core::LLVMPointerType(char, llvm_sys::AddressSpace::DATA);
            let ptr = llvm_sys::core::LLVMPointerType(char, 0); // 0 represents the default (DATA) address space
            let size =
                llvm_sys::core::LLVMIntTypeInContext(NonNull::from(llcx).as_ptr(), pointer_width);
            Types {
                double: &*llvm_sys::core::LLVMDoubleTypeInContext(NonNull::from(llcx).as_ptr()),
                char: &*char,
                int: &*llvm_sys::core::LLVMInt32TypeInContext(NonNull::from(llcx).as_ptr()),
                size: &*size,
                ptr: &*ptr,
                fat_ptr: ty_struct(
                    llcx,
                    "fat_ptr",
                    &[
                        &*ptr,
                        &*llvm_sys::core::LLVMInt64TypeInContext(NonNull::from(llcx).as_ptr()),
                    ],
                ),
                bool: &*llvm_sys::core::LLVMInt1TypeInContext(NonNull::from(llcx).as_ptr()),
                void: &*llvm_sys::core::LLVMVoidTypeInContext(NonNull::from(llcx).as_ptr()),
                null_ptr_val: &*llvm_sys::core::LLVMConstPointerNull(ptr),
            }
        }
    }
}
fn ty_struct<'ll>(
    llcx: &'ll llvm_sys::LLVMContext,
    name: &str,
    elements: &[&'ll Type],
) -> &'ll Type {
    let name = CString::new(name).unwrap();
    unsafe {
        let ty = llvm_sys::core::LLVMStructCreateNamed(NonNull::from(llcx).as_ptr(), name.as_ptr());

        // Convert &[&'ll Type] to Vec<*mut LLVMType>
        let mut element_ptrs: Vec<*mut llvm_sys::LLVMType> =
            elements.iter().map(|&e| e as *const _ as *mut _).collect();

        llvm_sys::core::LLVMStructSetBody(
            ty,
            element_ptrs.as_mut_ptr(),
            elements.len() as u32,
            0, //false
        );
        &*ty
    }
}

impl<'a, 'll> CodegenCx<'a, 'll> {
    #[inline(always)]
    pub fn ty_double(&self) -> &'ll Type {
        self.tys.double
    }
    #[inline(always)]
    pub fn ty_int(&self) -> &'ll Type {
        self.tys.int
    }
    #[inline(always)]
    pub fn ty_char(&self) -> &'ll Type {
        self.tys.char
    }
    #[inline(always)]
    pub fn ty_size(&self) -> &'ll Type {
        self.tys.size
    }
    #[inline(always)]
    pub fn ty_bool(&self) -> &'ll Type {
        self.tys.bool
    }
    #[inline(always)]
    pub fn ty_c_bool(&self) -> &'ll Type {
        self.tys.char
    }
    #[inline(always)]
    pub fn ty_ptr(&self) -> &'ll Type {
        self.tys.ptr
    }
    #[inline(always)]
    pub fn ty_void(&self) -> &'ll Type {
        self.tys.void
    }
    #[inline(always)]
    pub fn ty_fat_ptr(&self) -> &'ll Type {
        self.tys.fat_ptr
    }
    pub fn ty_aint(&self, bits: u32) -> &'ll Type {
        unsafe { &*llvm_sys::core::LLVMIntTypeInContext(NonNull::from(self.llcx).as_ptr(), bits) }
    }

    pub fn ty_struct(&self, name: &str, elements: &[&'ll Type]) -> &'ll Type {
        ty_struct(self.llcx, name, elements)
    }

    pub fn ty_func(&self, args: &[&'ll Type], ret: &'ll Type) -> &'ll Type {
        unsafe {
            let mut arg_ptrs: Vec<*mut llvm_sys::LLVMType> =
                args.iter().map(|&arg| arg as *const _ as *mut _).collect();

            &*llvm_sys::core::LLVMFunctionType(
                ret as *const _ as *mut _,
                arg_ptrs.as_mut_ptr(),
                args.len() as c_uint,
                FALSE,
            )
        }
    }
    pub fn ty_variadic_func(&self, args: &[&'ll Type], ret: &'ll Type) -> &'ll Type {
        unsafe {
            let mut arg_ptrs: Vec<*mut llvm_sys::LLVMType> =
                args.iter().map(|&arg| arg as *const _ as *mut _).collect();

            &*llvm_sys::core::LLVMFunctionType(
                ret as *const _ as *mut _,
                arg_ptrs.as_mut_ptr(),
                args.len() as c_uint,
                TRUE,
            )
        }
    }

    pub fn ty_array(&self, ty: &'ll Type, len: u32) -> &'ll Type {
        unsafe { &*llvm_sys::core::LLVMArrayType2(NonNull::from(ty).as_ptr(), len.into()) }
    }

    pub fn const_val(&self, val: &Const) -> &'ll Value {
        match *val {
            Const::Float(val) => self.const_real(val.into()),
            Const::Int(val) => self.const_int(val),
            Const::Bool(val) => self.const_bool(val),
            // Const::Complex(ref val) => self.const_cmplx(val),
            Const::Str(val) => self.const_str(val),
        }
    }

    /// # Safety
    /// indices must be valid and inbounds for the provided ptr
    /// The pointer must be a constant address
    pub unsafe fn const_gep(
        &self,
        elem_ty: &'ll Type,
        ptr: &'ll Value,
        indices: &[&'ll Value],
    ) -> &'ll Value {
        let mut index_ptrs: Vec<*mut llvm_sys::LLVMValue> =
            indices.iter().map(|&v| NonNull::from(v).as_ptr()).collect();

        &*llvm_sys::core::LLVMConstInBoundsGEP2(
            NonNull::from(elem_ty).as_ptr(),
            NonNull::from(ptr).as_ptr(),
            index_ptrs.as_mut_ptr(),
            indices.len() as u32,
        )
    }
    pub fn const_int(&self, val: i32) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(NonNull::from(self.ty_int()).as_ptr(), val as u64, TRUE)
        }
    }

    pub fn const_unsigned_int(&self, val: u32) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(NonNull::from(self.ty_int()).as_ptr(), val as u64, TRUE)
        }
    }

    pub fn const_isize(&self, val: isize) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(NonNull::from(self.ty_size()).as_ptr(), val as u64, TRUE)
        }
    }

    pub fn const_usize(&self, val: usize) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(
                NonNull::from(self.ty_size()).as_ptr(),
                val as u64,
                FALSE,
            )
        }
    }

    pub fn const_bool(&self, val: bool) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(
                NonNull::from(self.ty_bool()).as_ptr(),
                val as u64,
                FALSE,
            )
        }
    }

    pub fn const_c_bool(&self, val: bool) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(
                NonNull::from(self.ty_c_bool()).as_ptr(),
                val as u64,
                FALSE,
            )
        }
    }

    pub fn const_u8(&self, val: u8) -> &'ll Value {
        unsafe {
            &*llvm_sys::core::LLVMConstInt(
                NonNull::from(self.ty_c_bool()).as_ptr(),
                val as u64,
                FALSE,
            )
        }
    }

    pub fn const_real(&self, val: f64) -> &'ll Value {
        unsafe { &*llvm_sys::core::LLVMConstReal(NonNull::from(self.ty_double()).as_ptr(), val) }
    }

    pub fn const_arr(&self, elem_ty: &'ll Type, vals: &[&'ll Value]) -> &'ll Value {
        unsafe {
            let mut val_ptrs: Vec<*mut llvm_sys::LLVMValue> =
                vals.iter().map(|&v| NonNull::from(v).as_ptr()).collect();
            &*llvm_sys::core::LLVMConstArray2(
                NonNull::from(elem_ty).as_ptr(),
                val_ptrs.as_mut_ptr(),
                vals.len() as u64,
            )
        }
    }

    pub fn const_struct(&self, ty: &'ll Type, vals: &[&'ll Value]) -> &'ll Value {
        unsafe {
            let mut val_ptrs: Vec<*mut llvm_sys::LLVMValue> =
                vals.iter().map(|&v| NonNull::from(v).as_ptr()).collect();
            &*llvm_sys::core::LLVMConstNamedStruct(
                NonNull::from(ty).as_ptr(),
                val_ptrs.as_mut_ptr(),
                vals.len() as u32,
            )
        }
    }

    pub fn const_null_ptr(&self) -> &'ll Value {
        self.tys.null_ptr_val
    }

    pub fn const_undef(&self, t: &'ll Type) -> &'ll Value {
        unsafe { &*llvm_sys::core::LLVMGetUndef(NonNull::from(t).as_ptr()) }
    }

    pub fn val_ty(&self, v: &'ll Value) -> &'ll Type {
        unsafe { &*llvm_sys::core::LLVMTypeOf(NonNull::from(v).as_ptr()) }
    }
}
