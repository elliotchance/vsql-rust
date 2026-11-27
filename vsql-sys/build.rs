// build.rs compiles libvsql from the amalgamation (see
// scripts/update-amalgamation.sh) and links the libraries that V used for the
// target OS and architecture (build_flags.rs). amalgamation/libvsql.c has the C
// for every OS and architecture, and defines what the garbage collector and the
// platform need itself.

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

    // The OS and architecture names of V (and vsql.mk). The libc (such as
    // glibc or musl on Linux, or MSVC or MinGW on Windows) doesn't matter.
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os_arch = format!(
        "{}_{}",
        os,
        match arch.as_str() {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            other => other,
        }
    );
    let Some((_, libraries)) = BUILD_FLAGS.iter().find(|(name, _)| *name == os_arch) else {
        // The amalgamation only has the C that V generated on the OSes that
        // vsql's release is built on (not the BSDs, for example).
        let supported: Vec<_> = BUILD_FLAGS.iter().map(|(name, _)| *name).collect();
        panic!(
            "vsql-sys can't be built for {os} on {arch}: the vsql amalgamation only has the C \
             for {}",
            supported.join(", ")
        );
    };

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("amalgamation");
    let mut build = cc::Build::new();
    build
        .file(dir.join("libvsql.c"))
        .include(&dir)
        .warnings(false)
        .debug(false)
        // The C that V generates relies on signed overflow wrapping, and causes
        // many warnings.
        .flag("-fwrapv")
        .flag("-w");

    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
    if target_env == "musl" {
        // musl has no getcontext, and the garbage collector must find the
        // static data of a static binary with dl_iterate_phdr (or it aborts
        // with "Wrong DATASTART/END pair").
        build
            .define("NO_GETCONTEXT", None)
            .define("HAVE_DL_ITERATE_PHDR", None)
            .define("DL_ITERATE_PHDR_STRONG", None);
    }
    if target_env == "msvc" {
        // The C that V generates uses GNU C (such as statement expressions),
        // which MSVC can't compile, so it's compiled with clang-cl. For clang
        // on Windows, it doesn't include stdio.h. It declares the printf
        // functions itself, which the Universal CRT defines inline, so they're
        // linked from legacy_stdio_definitions.
        build.compiler(clang_cl());
        println!("cargo:rustc-link-lib=legacy_stdio_definitions");
    } else {
        build.flag("-std=gnu11");
        // The C that V generates for Linux passes a pthread_t as a void*,
        // which is an error from GCC 14 unless -Wno-int-conversion is given.
        build.flag_if_supported("-Wno-int-conversion");
    }
    build.compile("vsql");

    // musl's libc.a has dl, pthread and m in it. Linking them would take the
    // system's (glibc) static libraries instead.
    if target_env == "musl" {
        return;
    }
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
