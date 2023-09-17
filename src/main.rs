mod llvm;
mod html_gen;

use clap::Parser;
use llvm::{Function, Instr, Module, Value};
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::{collections::HashMap, process::Command, time::Instant};

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum LineStatus {
    NotInBinary,
    NoPanic,
    Panic,
}

#[derive(Debug)]
struct DataFile {
    lines: Vec<LineStatus>,
}
struct Data<'x> {
    files: HashMap<&'x str, DataFile>,
    no_of_panicky_fns: usize,
    panic_fns: Vec<Function<'x>>,
}

fn is_panicky(panic_fns: &[Function], instr: Instr) -> bool {
    match instr {
        Instr::Call(instr) => match instr.called_fn() {
            Value::Function(fun) => panic_fns.binary_search(&fun).is_ok(),
            _ => false,
        },
        _ => false,
    }
}
fn do_instr<'x>(data: &mut Data<'x>, instr: Instr<'x>) {
    let value = instr.as_value();
    let Some(debug_info) = value.debug_info() else {
        return;
    };

    let lines = &mut data
        .files
        .entry(debug_info.filename)
        .or_insert(DataFile { lines: Vec::new() })
        .lines;

    let line = debug_info.line as usize;
    if line >= lines.len() {
        lines.resize(line + 1, LineStatus::NotInBinary);
    }

    let the_line = &mut lines[line];
    if *the_line == LineStatus::Panic {
        return;
    }

    let is_panicky = is_panicky(&data.panic_fns, instr);
    *the_line = if is_panicky {
        LineStatus::Panic
    } else {
        LineStatus::NoPanic
    };
}

fn do_fn<'x>(data: &mut Data<'x>, original_fun: Function<'x>) {
    let mut bbs = original_fun.bbs();
    while let Some(bb) = bbs.next() {
        let mut instrs = bb.instrs();
        while let Some(instr) = instrs.next() {
            do_instr(data, instr);
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
#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short = 'r', long, default_value = "release")]
    profile: String,
    #[arg(short, long)]
    package: String,
    #[arg(short, long)]
    target: String,
    #[arg(short, long, default_value_t = false)]
    init: bool,
}

fn run_command(args: &[&str]) {
    let mut out = "Running command: ".to_string();
    for i in args {
        write!(&mut out, "{} ", i).unwrap();
    }
    println!("{}", out);

    let exe = args[0];
    let args = &args[1..];

    Command::new(exe)
        .args(args)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}
fn do_init(args: &Args) {
    let version = "nightly-2023-08-17";
    run_command(&["rustup", "install", version]);

    run_command(&[
        "rustup",
        "run",
        version,
        "cargo",
        "rustc",
        //
        "-p",
        &args.package,
        //
        "--profile",
        &args.profile,
        //
        "-Z",
        "build-std=std,core",
        "--target",
        &args.target,
        //
        "--",
        "--emit",
        "llvm-bc",
    ]);
}

fn main() {
    let time_total = Instant::now();

    let version = llvm::get_version();
    if version.major != 17 {
        panic!("LLVM version 17 is required");
    }

    let args = Args::parse();
    if args.init {
        do_init(&args);
    }

    let expected_path = format!("target/release/deps/{}.bc", args.package);
    if !Path::new(&expected_path).exists() {
        panic!("{} does not exist", expected_path);
    }

    let time = time_total;
    let module = Module::from_bc(&expected_path);
    println!("loaded module in {:?}", time.elapsed());

    let panic_fns = find_panic_fns(&module);
    print_panic_fns(&panic_fns);

    let data = &mut Data {
        files: HashMap::new(),
        no_of_panicky_fns: 0,
        panic_fns,
    };
    let mut fns = module.fns();
    while let Some(fun) = fns.next() {
        do_fn(data, fun);
    }

    // for i in data.files.keys() {
    //     println!("{}", i);
    // }

    let output_folder = "target/panicatorul";
    fs::create_dir_all(output_folder).unwrap();
    html_gen::gen(output_folder, &data.files);

    println!("no of files: {}", data.files.len());
    println!("no of panicky fns: {}", data.no_of_panicky_fns);
    println!("total time: {:?}", time_total.elapsed());
}

// https://stackoverflow.com/a/69048758/4091452
// RUSTFLAGS="--emit=llvm-bc" cargo build --release
// $env:RUSTFLAGS="--emit=llvm-bc"; cargo +nightly build --release -Z build-std --target x86_64-pc-windows-msvc

// llvm-link --only-needed target/x86_64-pc-windows-msvc/release/deps/*.bc > all.bc
// llvm-link target/release/deps/*.bc > all.bc
// llvm-dis all.bc -o all.ll
