mod llvm;

use std::time::Instant;

use llvm::Module;

fn main() {
    let time = Instant::now();
    let module = Module::from_bc(r#"D:\tmp\hello_world\all.bc"#);
    println!("Loaded module in {:?}", time.elapsed());

    let mut fns = module.fns();
    let mut count = 0;
    while let Some(fun) = fns.next() {
        count += 1;
    }
    println!("{}fns", count);
}

// https://stackoverflow.com/a/69048758/4091452
// RUSTFLAGS="--emit=llvm-bc" cargo build --release
// llvm-link target/release/deps/*.bc > all.bc
// llvm-dis all.bc -o all.ll
