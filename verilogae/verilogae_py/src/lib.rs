#[macro_use]
mod offsets;
#[macro_use]
mod ffi;
mod load;
mod model;
mod numpy;
mod typeref;
mod unicode;
mod util;

use std::os::raw::{c_char, c_int};
use std::ptr;

use pyo3_ffi::*;

use crate::load::{load_info_py, load_py, load_vfs};
use crate::model::{VAE_FUNCTION_TY, VAE_MODEL_TY, VAE_PARAM_TY};
use crate::typeref::init_typerefs;

const FUN_FLAG: c_int = METH_VARARGS;

static mut FUNCTIONS: [PyMethodDef; 4] = unsafe {
    [
    PyMethodDef {
            ml_name: c"load".as_ptr(),
            ml_meth: PyMethodDefPointer{PyCFunctionWithKeywords: load_py},
            ml_flags: FUN_FLAG | METH_KEYWORDS,
            ml_doc: c"loads a Verilog-A model by either loading it from the object cache or compiling it".as_ptr(),
    },
    PyMethodDef {
            ml_name: c"load_info".as_ptr(),
            ml_meth: PyMethodDefPointer{PyCFunctionWithKeywords: load_info_py},
            ml_flags: FUN_FLAG | METH_KEYWORDS,
            ml_doc: c"loads information about Verilog-A model by either loading it from the object cache or compiling it\nThis function does not compile retrieved functions.\nThis allows for much faster compile times.\nModelsCompiled with this function lack the `functions` attribute.".as_ptr(),
    },
    PyMethodDef {
            ml_name: c"export_vfs".as_ptr(),
            ml_meth: PyMethodDefPointer{PyCFunctionWithKeywords: load_vfs},
            ml_flags: FUN_FLAG | METH_KEYWORDS,
            ml_doc: c"runs the preprocessor on a Verilog-A file and exports a dict with all files.\nThe result of this functions can be passed to other functions `vfs` argument".as_ptr(),
    },
    zero!(PyMethodDef)
]
};

#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[allow(static_mut_refs)]
#[no_mangle]
#[cold]
pub unsafe extern "C" fn PyInit_verilogae() -> *mut PyObject {
    let init = PyModuleDef {
        m_base: PyModuleDef_HEAD_INIT,
        m_name: c"verilogae".as_ptr(),
        m_doc: std::ptr::null(),
        m_size: 0,
        m_methods: FUNCTIONS.as_mut_ptr(),
        m_slots: std::ptr::null_mut(),
        m_traverse: None,
        m_clear: None,
        m_free: None,
    };

    if PyType_Ready(std::ptr::addr_of_mut!(VAE_MODEL_TY)) < 0 {
        return ptr::null_mut();
    }

    if PyType_Ready(std::ptr::addr_of_mut!(VAE_FUNCTION_TY)) < 0 {
        return ptr::null_mut();
    }

    if PyType_Ready(std::ptr::addr_of_mut!(VAE_PARAM_TY)) < 0 {
        return ptr::null_mut();
    }

    let mptr = PyModule_Create(Box::into_raw(Box::new(init)));
    init_typerefs();
    let version = env!("CARGO_PKG_VERSION");
    PyModule_AddObject(
        mptr,
        c"__version__".as_ptr(),
        PyUnicode_FromStringAndSize(version.as_ptr() as *const c_char, version.len() as isize),
    );

    let all = [c"__all__", c"__version__", c"load", c"load_info", c"export_vfs"];

    let pyall = PyTuple_New(all.len() as isize);
    for (i, obj) in all.iter().enumerate() {
        PyTuple_SET_ITEM(pyall, i as isize, PyUnicode_InternFromString(obj.as_ptr()))
    }

    PyModule_AddObject(mptr, c"__all__".as_ptr(), pyall);

    mptr
}
