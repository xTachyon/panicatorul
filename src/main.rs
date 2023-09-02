mod llvm;

use llvm::{Function, Instr, Module, Value};
use std::time::Instant;

use crate::llvm::DebugInfo;

// type SmallString1024 = SmallString<[u8; 1024]>;

fn find_panic_fns(module: &Module) -> Vec<Function> {
    let mut result = Vec::new();

    let mut fns = module.fns();
    while let Some(fun) = fns.next() {
        if !fun.name().starts_with("_ZN4core9panicking") {
            continue;
        }
        result.push(fun);
    }

    result.sort();
    result
}

fn do_fn(original_fun: Function, panic_fns: &[Function], no_of_panicky_fns: &mut usize) {
    let mut bbs = original_fun.bbs();
    while let Some(bb) = bbs.next() {
        let mut instrs = bb.instrs();
        while let Some(generic_instr) = instrs.next() {
            match &generic_instr {
                Instr::Call(instr) => match instr.called_fn() {
                    Value::Function(fun) => {
                        if panic_fns.contains(&fun) {
                            *no_of_panicky_fns += 1;

                            let value = generic_instr.as_value();
                            let debug_info = value.debug_info().unwrap_or(DebugInfo {
                                filename: "<unknown>",
                                line: 0,
                            });
                            println!("{}:{}", debug_info.filename, debug_info.line);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn print_panic_fns(panic_fns: &[Function]) {
    println!("panic fns: ");
    for i in panic_fns {
        println!("{}", i.name());
    }
    println!();
}

fn main() {
    let time_total = Instant::now();
    let time = time_total;
    let module = Module::from_bc(r#"D:\tmp\hello_world\all.bc"#);
    println!("loaded module in {:?}", time.elapsed());

    let panic_fns = find_panic_fns(&module);
    print_panic_fns(&panic_fns);

    let mut no_of_panicky_fns = 0;
    let mut fns = module.fns();
    while let Some(fun) = fns.next() {
        do_fn(fun, &panic_fns, &mut no_of_panicky_fns);
    }

    println!("no of panicky fns: {}", no_of_panicky_fns);
    println!("total time: {:?}", time_total.elapsed());
}

// https://stackoverflow.com/a/69048758/4091452
// RUSTFLAGS="--emit=llvm-bc" cargo build --release
// llvm-link target/release/deps/*.bc > all.bc
// llvm-dis all.bc -o all.ll
