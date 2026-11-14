# iefixup

A `WININET.DLL` wrapper for Internet Explorer 3 on Windows 95 that rewrites
modern HTML on the fly so that IE 3 can render it. Combined with
[schannel9x](../schannel9x) this makes sites such as https://mbbs.no and
https://tauslipp.no usable.

Every export is forwarded to the real `WININET.DLL` in the system directory.
`InternetReadFile()` and `InternetQueryDataAvailable()` are intercepted for
`text/html` responses, which are streamed through a rewriter that:

- transcodes UTF-8 to Windows-1252, mapping typographic punctuation to ASCII;
- drops `<script>`, `<style>`, `<svg>`, `<template>` and similar elements along
  with their content, plus `<link>` and comments;
- turns HTML5 sectioning elements (`header`, `nav`, `main`, `section`, ...)
  into `div`;
- keeps only attributes that IE 3 understands;
- replaces SVG images with their alt text.

URLMON drains exactly the number of bytes WININET reports as available, so
every rewritten chunk is padded with spaces to its original length, placed
inside the last tag of the chunk where they are invisible.

## Building

    make

Needs rustup and `i686-w64-mingw32-gcc`. The pinned nightly toolchain builds
`core` and `alloc` for `i586-pc-windows-gnu.json`, an i586 variant of Rust's
MinGW target, as Windows 95 era CPUs lack SSE2. The result is
`target/WININET.DLL`, which only imports from `KERNEL32`.

Debug builds log to QEMU's `isa-debugcon` port `0xE9`; build with
`cargo build --no-default-features` to drop that. `make check` runs the tests
on the host.

## Installing

Copy `build/WININET.DLL` next to `IEXPLORE.EXE`, typically
`C:\Program Files\Internet Explorer`. Windows searches the application's
directory first, so only IE picks it up. Delete it to uninstall.

## Limitations

IE 3 cannot decode PNG or SVG, and images are shown at their natural size.

## License

MIT.
