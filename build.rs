use std::env;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

const HOOKS: &[&str] = &[
    "InternetQueryDataAvailable",
    "InternetReadFile",
    "InternetCloseHandle",
];

const LINK_ARGS: &[&str] = &[
    "-nostartfiles",
    "-nostdlib",
    "-Wl,--entry,_DllMain@12",
    "-Wl,--enable-stdcall-fixup",
    "-Wl,--gc-sections",
    "-Wl,--image-base,0x7a800000",
    "-Wl,--major-os-version,4",
    "-Wl,--minor-os-version,0",
    "-Wl,--major-subsystem-version,4",
    "-Wl,--minor-subsystem-version,0",
    "-Wl,--no-insert-timestamp",
    "-Wl,--disable-dynamicbase",
    "-Wl,--disable-nxcompat",
    "-lkernel32",
    "-luser32",
];

fn main() {
    println!("cargo::rerun-if-changed=src/wininet.exports");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let exports = parse_exports(&fs::read_to_string("src/wininet.exports").unwrap());
    let forwarded: Vec<&str> = exports
        .iter()
        .map(|(_, name)| name.as_str())
        .filter(|name| !HOOKS.contains(name))
        .collect();

    fs::write(out_dir.join("forward.rs"), render_forwarders(&forwarded)).unwrap();

    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let def_path = out_dir.join("wininet.def");
        fs::write(&def_path, render_def(&exports)).unwrap();
        println!("cargo::rustc-cdylib-link-arg={}", def_path.display());
        for arg in LINK_ARGS {
            println!("cargo::rustc-cdylib-link-arg={arg}");
        }
    }
}

fn parse_exports(text: &str) -> Vec<(u32, String)> {
    text.lines()
        .map(|line| {
            let (ordinal, name) = line.split_once(' ').unwrap();
            (ordinal.parse().unwrap(), name.to_string())
        })
        .collect()
}

fn render_forwarders(forwarded: &[&str]) -> String {
    let mut out = String::new();
    writeln!(
        out,
        "pub static FORWARD_NAMES: [&[u8]; {}] = [",
        forwarded.len()
    )
    .unwrap();
    for name in forwarded {
        writeln!(out, "    b\"{name}\\0\",").unwrap();
    }
    writeln!(out, "];\n").unwrap();
    writeln!(
        out,
        "pub static mut FORWARD_TARGETS: [usize; {}] = [0; {}];\n",
        forwarded.len(),
        forwarded.len()
    )
    .unwrap();
    writeln!(out, "#[cfg(windows)]").unwrap();
    writeln!(out, "core::arch::global_asm!(").unwrap();
    for (index, name) in forwarded.iter().enumerate() {
        writeln!(
            out,
            "    \".globl _{name}\\n_{name}:\\n    jmp dword ptr [{{targets}} + {}]\\n\",",
            index * 4
        )
        .unwrap();
    }
    writeln!(out, "    targets = sym FORWARD_TARGETS,").unwrap();
    writeln!(out, ");").unwrap();
    out
}

fn render_def(exports: &[(u32, String)]) -> String {
    let mut out = String::from("LIBRARY WININET.DLL\nEXPORTS\n");
    for (ordinal, name) in exports {
        writeln!(out, "  {name} @{ordinal}").unwrap();
    }
    out
}
