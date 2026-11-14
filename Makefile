HOST := $(shell rustc -vV | sed -n 's/^host: //p')

all:
	cargo build --release -Zbuild-std=core,alloc -Zbuild-std-features=compiler-builtins-mem
	i686-w64-mingw32-strip -o target/WININET.DLL target/i586-pc-windows-gnu/release/iefixup.dll

check:
	cargo test --target $(HOST)

clean:
	cargo clean

.PHONY: all check clean
