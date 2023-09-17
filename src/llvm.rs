use std::slice;
use std::str;
use std::{ffi::CStr, marker::PhantomData, ptr::null_mut};

use llvm_sys::core::LLVMGetCalledValue;
use llvm_sys::core::LLVMGetDebugLocFilename;
use llvm_sys::core::LLVMGetDebugLocLine;
use llvm_sys::core::LLVMGetFirstBasicBlock;
use llvm_sys::core::LLVMGetFirstInstruction;
use llvm_sys::core::LLVMGetInstructionOpcode;
use llvm_sys::core::LLVMGetNextBasicBlock;
use llvm_sys::core::LLVMGetNextInstruction;
use llvm_sys::core::LLVMGetVersion;
use llvm_sys::core::LLVMIsAFunction;
use llvm_sys::prelude::LLVMBasicBlockRef;
use llvm_sys::LLVMOpcode;
use llvm_sys::{
    bit_reader::LLVMParseBitcodeInContext2,
    core::{
        LLVMContextCreate, LLVMContextDispose, LLVMCreateMemoryBufferWithContentsOfFile,
        LLVMDisposeMemoryBuffer, LLVMDisposeModule, LLVMGetFirstFunction, LLVMGetNextFunction,
        LLVMGetValueName2,
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
        let ctx = unsafe { LLVMGetFirstFunction(self.module) };
        FunctionIterator {
            ctx,
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

unsafe fn from_c_str<'x, S: Into<u32>>(ptr: *const i8, size: S) -> &'x str {
    let size = size.into() as usize;
    let name = unsafe { slice::from_raw_parts(ptr.cast(), size) };
    str::from_utf8(name).unwrap()
}

pub struct FunctionIterator<'x> {
    ctx: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> FunctionIterator<'x> {
    pub fn next(&mut self) -> Option<Function<'x>> {
        let current = self.ctx;
        if current.is_null() {
            return None;
        }
        self.ctx = unsafe { LLVMGetNextFunction(self.ctx) };

        Some(Function {
            ctx: current,
            _p: PhantomData,
        })
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct Function<'x> {
    ctx: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}

impl<'x> Function<'x> {
    pub fn name(&self) -> &str {
        let name = unsafe {
            let mut size = 0;
            let name = LLVMGetValueName2(self.ctx, &mut size);
            slice::from_raw_parts(name.cast(), size)
        };
        str::from_utf8(name).unwrap()
    }

    pub fn bbs(self) -> BasicBlockIterator<'x> {
        let bb = unsafe { LLVMGetFirstBasicBlock(self.ctx) };
        BasicBlockIterator {
            ctx: bb,
            _p: PhantomData,
        }
    }

    // pub fn as_value(&self) -> Value {
    //     Value::Function(Function {
    //         ctx: self.ctx,
    //         _p: PhantomData,
    //     })
    // }
}

pub struct BasicBlockIterator<'x> {
    ctx: LLVMBasicBlockRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> BasicBlockIterator<'x> {
    pub fn next(&mut self) -> Option<BasicBlock<'x>> {
        let current = self.ctx;
        if current.is_null() {
            return None;
        }
        self.ctx = unsafe { LLVMGetNextBasicBlock(self.ctx) };

        Some(BasicBlock {
            ctx: current,
            _p: PhantomData,
        })
    }
}
pub struct BasicBlock<'x> {
    ctx: LLVMBasicBlockRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> BasicBlock<'x> {
    pub fn instrs(&self) -> InstrIterator<'x> {
        let instr = unsafe { LLVMGetFirstInstruction(self.ctx) };
        InstrIterator {
            ctx: instr,
            _p: PhantomData,
        }
    }
}

pub struct InstrIterator<'x> {
    ctx: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> InstrIterator<'x> {
    pub fn next(&mut self) -> Option<Instr<'x>> {
        let current = self.ctx;
        if current.is_null() {
            return None;
        }
        self.ctx = unsafe { LLVMGetNextInstruction(self.ctx) };

        Some(unsafe { Instr::new(current) })
    }
}

#[derive(Copy, Clone)]
pub struct CallInstr<'x> {
    ctx: LLVMValueRef,
    _p: PhantomData<&'x ()>,
}
impl<'x> CallInstr<'x> {
    pub fn called_fn(&self) -> Value {
        unsafe {
            let called_fn = LLVMGetCalledValue(self.ctx);
            Value::new(called_fn)
        }
    }
}

#[derive(Copy, Clone)]
pub enum Instr<'x> {
    Call(CallInstr<'x>),
    Other(LLVMValueRef),
}
impl<'x> Instr<'x> {
    unsafe fn new(instr: LLVMValueRef) -> Instr<'x> {
        use Instr::*;

        let opcode = LLVMGetInstructionOpcode(instr);
        match opcode {
            LLVMOpcode::LLVMCall => Call(CallInstr {
                ctx: instr,
                _p: PhantomData,
            }),
            _ => Other(instr),
        }
    }
    fn value_raw(&self) -> LLVMValueRef {
        match self {
            Instr::Call(x) => x.ctx,
            Instr::Other(x) => *x,
        }
    }
    pub fn as_value(self) -> Value<'x> {
        Value::Other(self.value_raw())
    }
    // pub fn dump(&self) {
    //     let ctx = match self {
    //         Instr::Call(x) => x.ctx,
    //         Instr::Other(x) => *x,
    //     };
    //     unsafe { LLVMDumpValue(ctx) };
    //     println!();
    // }
}

pub enum Value<'x> {
    Function(Function<'x>),
    Other(LLVMValueRef),
}
impl<'x> Value<'x> {
    unsafe fn new(value: LLVMValueRef) -> Value<'x> {
        if !LLVMIsAFunction(value).is_null() {
            Value::Function(Function {
                ctx: value,
                _p: PhantomData,
            })
        } else {
            Value::Other(value)
        }
    }

    fn value_raw(&self) -> LLVMValueRef {
        match self {
            Value::Function(x) => x.ctx,
            Value::Other(x) => *x,
        }
    }

    pub fn debug_info(self) -> Option<DebugInfo<'x>> {
        unsafe {
            let raw_value = self.value_raw();

            let mut size = 0;
            let name = LLVMGetDebugLocFilename(raw_value, &mut size);
            let filename = from_c_str(name, size);

            let line = LLVMGetDebugLocLine(raw_value);

            if filename.is_empty() && line == 0 {
                None
            } else {
                Some(DebugInfo { filename, line })
            }
        }
    }
}

pub struct DebugInfo<'x> {
    pub filename: &'x str,
    pub line: u32,
}

pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
pub fn get_version() -> Version {
    let mut major = 0;
    let mut minor = 0;
    let mut patch = 0;
    unsafe {
        LLVMGetVersion(&mut major, &mut minor, &mut patch);
    }
    Version {
        major,
        minor,
        patch,
    }
}
