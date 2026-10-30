// build.rs compiles libvsql from the amalgamation (see
// scripts/update-amalgamation.sh) with the defines and libraries that V used
// for the target OS and architecture (build_flags.rs).

use std::env;
use std::path::{Path, PathBuf};

include!("build_flags.rs");

fn main() {
    println!("cargo:rerun-if-changed=amalgamation");
    println!("cargo:rerun-if-changed=build_flags.rs");

    // docs.rs only builds the documentation, which doesn't need libvsql.
    if env::var_os("DOCS_RS").is_some() {
        return;
    }

    let os = match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "macos" => "macos",
        "linux" => "linux",
        "windows" => "windows",
        other => panic!("vsql isn't supported on {other}"),
    };
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => panic!("vsql isn't supported on {other}"),
    };
    let os_arch = format!("{os}_{arch}");
    let (_, defines, libraries) = BUILD_FLAGS
        .iter()
        .find(|(name, _, _)| *name == os_arch)
        .unwrap_or_else(|| panic!("there's no libvsql C for {os_arch}"));

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("amalgamation");
    let mut build = cc::Build::new();
    build
        // The garbage collector and V's helpers have to be compiled separately
        // from libvsql.c (see amalgamation/vsql_runtime.c).
        .file(dir.join("libvsql.c"))
        .file(dir.join("vsql_runtime.c"))
        .include(&dir)
        .warnings(false)
        .debug(false);
    for define in *defines {
        match define.split_once('=') {
            Some((name, value)) => build.define(name, value),
            None => build.define(define, None),
        };
    }

    let msvc = env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc";
    if msvc {
        // The C that V generates uses GNU C (such as statement expressions),
        // which MSVC can't compile, so it's compiled with clang-cl. For clang
        // on Windows, it doesn't include stdio.h. It declares the printf
        // functions itself, which the Universal CRT defines inline, so they're
        // linked from legacy_stdio_definitions. And it defines the buffering
        // modes for setvbuf with glibc's values unless they're already defined,
        // which the Universal CRT rejects (as an invalid parameter, which kills
        // the process).
        build.compiler(clang_cl());
        build.define("_IOFBF", "0x0000");
        build.define("_IOLBF", "0x0040");
        build.define("_IONBF", "0x0004");
        println!("cargo:rustc-link-lib=legacy_stdio_definitions");
    } else {
        build.flag("-std=gnu11");
        // The C that V generates for Linux passes a pthread_t as a void*,
        // which is an error from GCC 14 unless -Wno-int-conversion is given.
        build.flag_if_supported("-Wno-int-conversion");
    }
    build.compile("vsql");

    for library in *libraries {
        println!("cargo:rustc-link-lib={library}");
    }
}

/// clang-cl from VSQL_CLANG_CL, the PATH, or LLVM's default folder.
fn clang_cl() -> PathBuf {
    println!("cargo:rerun-if-env-changed=VSQL_CLANG_CL");
    if let Some(path) = env::var_os("VSQL_CLANG_CL") {
        return path.into();
    }
    let path = env::var_os("PATH").unwrap_or_default();
    for dir in env::split_paths(&path) {
        let candidate = dir.join("clang-cl.exe");
        if candidate.is_file() {
            return candidate;
        }
    }
    let default = PathBuf::from(r"C:\Program Files\LLVM\bin\clang-cl.exe");
    if default.is_file() {
        return default;
    }
    panic!(
        "vsql must be compiled with clang-cl on Windows (install LLVM, or set VSQL_CLANG_CL to \
         the path of clang-cl.exe)"
    );
}
