use std::{ffi::CStr, marker::PhantomData, ptr::null_mut};

use llvm_sys::{
    bit_reader::LLVMParseBitcodeInContext2,
    core::{
        LLVMContextCreate, LLVMContextDispose, LLVMCreateMemoryBufferWithContentsOfFile,
        LLVMDisposeMemoryBuffer, LLVMDisposeModule, LLVMGetFirstFunction, LLVMGetNextFunction,
    },
    prelude::{LLVMContextRef, LLVMMemoryBufferRef, LLVMModuleRef, LLVMValueRef},
};
use smallstr::SmallString;

pub struct Module {
    ctx: LLVMContextRef,
    module: LLVMModuleRef,
}
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeModule(self.module);
            LLVMContextDispose(self.ctx);
        }
    }
}
impl Module {
    pub fn from_bc(path: &str) -> Module {
        let buffer = MemoryBuffer::new(path);

        let ctx = unsafe { LLVMContextCreate() };
        let mut module = null_mut();
        let ret = unsafe { LLVMParseBitcodeInContext2(ctx, buffer.ctx, &mut module) };
        if ret != 0 {
            panic!("Failed to parse bitcode");
        }

        Module { ctx, module }
    }

    pub fn fns(&self) -> FunctionIterator {
        let fun = unsafe { LLVMGetFirstFunction(self.module) };
        FunctionIterator {
            fun,
            _p: PhantomData,
        }
    }
}

struct MemoryBuffer {
    ctx: LLVMMemoryBufferRef,
}
impl Drop for MemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeMemoryBuffer(self.ctx);
        }
    }
}
impl MemoryBuffer {
    fn new(path: &str) -> MemoryBuffer {
        let path = c_str(path);
        let mut ctx = null_mut();
        let mut message = null_mut();
        let ret = unsafe {
            LLVMCreateMemoryBufferWithContentsOfFile(path.as_ptr().cast(), &mut ctx, &mut message)
        };
        if ret != 0 {
            // TODO: leak, but does it really matter if we panic
            let message = unsafe { CStr::from_ptr(message) };
            panic!(
                "Failed to create memory buffer: {}",
                message.to_string_lossy()
            );
        }
        MemoryBuffer { ctx }
    }
}

fn c_str(s: &str) -> SmallString<[u8; 128]> {
    let mut r = SmallString::from_str(s);
    r.push('\0');
    r
}

pub struct FunctionIterator<'x> {
    fun: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> FunctionIterator<'x> {
    pub fn next(&mut self) -> Option<Function<'x>> {
        let current = self.fun;
        if current.is_null() {
            return None;
        }
        self.fun = unsafe { LLVMGetNextFunction(self.fun) };

        Some(Function {
            fun: current,
            _p: PhantomData,
        })
    }
}

pub struct Function<'x> {
    fun: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}
