use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::Once;

use pyo3_ffi::{
    PyCapsule_GetPointer, PyDict_New, PyErr_Clear, PyFloat_FromDouble, PyImport_ImportModule,
    PyList_New, PyLong_FromLongLong, PyMapping_GetItemString, PyObject, PyObject_GenericGetDict,
    PyObject_GetAttrString, PyTypeObject, PyUnicode_InternFromString, PyUnicode_New, Py_XDECREF,
    Py_intptr_t,
};

const NPY_INT: c_int = 5;
const NPY_CDOUBLE: c_int = 12;

type PyArrayNew = extern "C" fn(
    subtype: *mut PyTypeObject,
    descr: *mut PyObject,
    nd: c_int,
    dims: *mut Py_intptr_t,
    strides: *mut Py_intptr_t,
    data: *mut c_void,
    flags: c_int,
    obj: *mut PyObject,
) -> *mut PyObject;

pub static mut NUMPY_ARR_TYPE: Option<*mut PyTypeObject> = None;
pub static mut NUMPY_API: Option<PyArrayNew> = None;
pub static mut NUMPY_CDOUBLE_DESCR: *mut PyObject = std::ptr::null_mut();
pub static mut NUMPY_INT_DESCR: *mut PyObject = std::ptr::null_mut();

pub static mut PATHLIB_PATH: *mut PyTypeObject = std::ptr::null_mut::<PyTypeObject>();
// pub static mut STR_TYPE: *mut PyTypeObject = 0 as *mut PyTypeObject;
pub static mut INT_TYPE: *mut PyTypeObject = std::ptr::null_mut::<PyTypeObject>();
pub static mut FLOAT_TYPE: *mut PyTypeObject = std::ptr::null_mut::<PyTypeObject>();
pub static mut LIST_TYPE: *mut PyTypeObject = std::ptr::null_mut::<PyTypeObject>();
pub static mut DICT_TYPE: *mut PyTypeObject = std::ptr::null_mut::<PyTypeObject>();
pub static mut ARRAY_STRUCT_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();
pub static mut EMPTY_UNICODE: *mut PyObject = std::ptr::null_mut::<PyObject>();
// Internted arguments so that kwargs check are simple pointer comparisons
pub static mut MODULE_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();
pub static mut VFS_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();
pub static mut VOLTAGES_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();
pub static mut CURRENTS_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();
pub static mut TEMPERATURE_STR: *mut PyObject = std::ptr::null_mut::<PyObject>();

static INIT: Once = Once::new();

#[cold]
pub fn init_typerefs() {
    INIT.call_once(|| unsafe {
        DICT_TYPE = (*PyDict_New()).ob_type;
        LIST_TYPE = (*PyList_New(0)).ob_type;
        INT_TYPE = (*PyLong_FromLongLong(0)).ob_type;
        FLOAT_TYPE = (*PyFloat_FromDouble(0.0)).ob_type;
        NUMPY_ARR_TYPE = load_numpy_types();

        MODULE_STR = PyUnicode_InternFromString(c"module".as_ptr());
        VFS_STR = PyUnicode_InternFromString(c"vfs".as_ptr());
        VOLTAGES_STR = PyUnicode_InternFromString(c"voltages".as_ptr());
        CURRENTS_STR = PyUnicode_InternFromString(c"currents".as_ptr());
        TEMPERATURE_STR = PyUnicode_InternFromString(c"temperature".as_ptr());
        EMPTY_UNICODE = PyUnicode_New(0, 255);

        ARRAY_STRUCT_STR = PyUnicode_InternFromString(c"__array_struct__".as_ptr());
        if let Some(numpy_api) = get_numpy_api() {
            let api = *(numpy_api.offset(94) as *const PyArrayNew);
            let py_array_descr_from_type =
                *(numpy_api.offset(45) as *const fn(type_: c_int) -> *mut PyObject);

            NUMPY_CDOUBLE_DESCR = py_array_descr_from_type(NPY_CDOUBLE);
            assert!(!NUMPY_CDOUBLE_DESCR.is_null());
            NUMPY_INT_DESCR = py_array_descr_from_type(NPY_INT);
            assert!(!NUMPY_INT_DESCR.is_null());

            NUMPY_API = Some(api);
        }
        let pathlib = PyImport_ImportModule(c"pathlib".as_ptr());
        assert!(!pathlib.is_null(), "failed to import pathlib!");
        PATHLIB_PATH = lookup_module_type(pathlib, c"Path".as_ptr());
    });
}

#[cold]
unsafe fn lookup_module_type(module: *mut PyObject, name: *const c_char) -> *mut PyTypeObject {
    let mod_dict = PyObject_GenericGetDict(module, std::ptr::null_mut());
    let ptr = PyMapping_GetItemString(mod_dict, name);
    Py_XDECREF(ptr);
    Py_XDECREF(mod_dict);
    ptr as *mut PyTypeObject
}

#[cold]
unsafe fn load_numpy_types() -> Option<*mut PyTypeObject> {
    let numpy = PyImport_ImportModule(c"numpy".as_ptr());
    if numpy.is_null() {
        PyErr_Clear();
        return None;
    }
    let array = lookup_module_type(numpy, c"ndarray".as_ptr());
    Py_XDECREF(numpy);
    Some(array)
}

#[cold]
fn get_numpy_api() -> Option<*const *const c_void> {
    unsafe {
        let numpy = PyImport_ImportModule(c"numpy.core.multiarray".as_ptr());
        if numpy.is_null() {
            PyErr_Clear();
            return None;
        }
        let capsule = PyObject_GetAttrString(numpy as _, c"_ARRAY_API".as_ptr());
        if capsule.is_null() {
            PyErr_Clear();
            return None;
        }
        Some(PyCapsule_GetPointer(capsule, ptr::null_mut()) as _)
    }
}
