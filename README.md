# iefixup

A `WININET.DLL` wrapper for Internet Explorer 5.5 on Windows 95 that rewrites
modern HTML and CSS on the fly so that IE 5.5 can render it. Combined with
[schannel9x](../schannel9x) this makes sites such as https://mbbs.no and
https://tauslipp.no usable.

Every export is forwarded to the real WININET, renamed to `WININETR.DLL`.
Requests go out without `Accept-Encoding`, as IE would otherwise ask for gzip,
which it decodes after WININET. Uncompressed `text/html` and `text/css`
responses are streamed through a rewriter as they are read.

HTML:

- transcodes UTF-8 to Windows-1252, mapping typographic punctuation to ASCII;
- drops `<script>`, `<svg>` and `<template>` along with their content, and the
  tags of `<noscript>`, `<picture>`, `<video>` and similar elements while
  keeping their fallback content;
- turns HTML5 sectioning elements (`header`, `nav`, `main`, `section`, ...)
  into `div`, adding an `ie-` class such as `ie-nav` for style rules to match;
- rewrites `<style>` contents and `style` attributes like stylesheets;
- replaces SVG images with their alt text.

CSS:

- resolves custom properties declared on `:root`;
- turns `rgba()`, `hsl()` and 4 or 8 digit hex colors into opaque colors,
  dropping mostly transparent ones, and gradients into their first color;
- computes `rem`, viewport units, `calc()`, `min()`, `max()` and `clamp()`,
  taking percentages relative to the screen width;
- maps `inset` to offsets and `opacity` to `visibility`;
- applies `@media` blocks that match the screen width, keeps `@import` and
  drops other at-rules;
- drops selectors that IE 5.5 cannot parse, and treats child combinators as
  descendant combinators.

MSHTML reads stylesheets from the cache file rather than through WININET, so
once a stylesheet is fully read, its cache file is replaced with the rewritten
one.

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

The real `WININET.DLL` is loaded by background processes such as WebCheck at
startup, and Windows 95 shares loaded modules by name, so iefixup has to take
its place in the system directory. Copy `target/WININET.DLL` there as
`IEFIXUP.DLL` and have Windows swap the files at the next boot through
`C:\WINDOWS\WININIT.INI`:

    [rename]
    C:\WINDOWS\SYSTEM\WININETR.DLL=C:\WINDOWS\SYSTEM\WININET.DLL
    C:\WINDOWS\SYSTEM\WININET.DLL=C:\WINDOWS\SYSTEM\IEFIXUP.DLL

To uninstall, rename `WININETR.DLL` back the same way.

## Limitations

Layout features without an IE 5.5 equivalent, such as flexbox, grid and
`aspect-ratio`, are left for IE to ignore. Pages that only work with
JavaScript stay empty.

## License

MIT.
