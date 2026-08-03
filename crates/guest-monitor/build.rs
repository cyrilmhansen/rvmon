use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repository_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap();
    let source = repository_root.join("examples/minibasic-asm/payload-repl.rv");
    let builder = repository_root.join("scripts/build-minibasic-asm-payload.sh");
    let modules = [
        "examples/minibasic-asm/modules/00_data_bootstrap.rv",
        "examples/minibasic-asm/modules/10_repl_and_dispatch.rv",
        "examples/minibasic-asm/modules/20_expression.rv",
        "examples/minibasic-asm/modules/30_arrays_and_functions.rv",
        "examples/minibasic-asm/modules/40_strings_and_tables.rv",
        "examples/minibasic-asm/modules/90_session.rv",
    ];
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("minibasic-payload");

    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", builder.display());
    for module in modules {
        println!(
            "cargo:rerun-if-changed={}",
            repository_root.join(module).display()
        );
    }

    let output = Command::new(&builder)
        .current_dir(repository_root)
        .env("MINIBASIC_ASM_OUTPUT_DIR", &output_dir)
        .output()
        .unwrap_or_else(|error| panic!("cannot run {}: {error}", builder.display()));
    if !output.status.success() {
        panic!(
            "MiniBASIC assembly payload generation failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let code = output_dir.join("minibasic-payload-asm.bin");
    let data = output_dir.join("minibasic-payload-asm-data.bin");
    if !code.is_file() || !data.is_file() {
        panic!("MiniBASIC payload generator did not produce code and data images");
    }
    println!(
        "cargo:rustc-env=RVMON_MINIBASIC_ASM_CODE={}",
        code.display()
    );
    println!(
        "cargo:rustc-env=RVMON_MINIBASIC_ASM_DATA={}",
        data.display()
    );
}
