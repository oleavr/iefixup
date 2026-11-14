CC      := i686-w64-mingw32-gcc
STRIP   := i686-w64-mingw32-strip
PYTHON  := python3
BUILD   := build
DEBUG   ?= 1

HOOKS := InternetQueryDataAvailable InternetReadFile InternetCloseHandle

CFLAGS := -Os -g -march=i586 -mtune=pentiumpro \
	-ffunction-sections -fdata-sections \
	-fno-asynchronous-unwind-tables -fno-stack-protector -fno-ident \
	-fno-tree-loop-distribute-patterns \
	-Wall -Wno-unused-parameter \
	-Isrc -I$(BUILD) \
	-D_WIN32_WINNT=0x0400 -DWINVER=0x0400 \
	-DIEFIXUP_DEBUG=$(DEBUG)

LDFLAGS := -shared -nostdlib -nostartfiles \
	-Wl,--entry,_DllMain@12 -Wl,--enable-stdcall-fixup -Wl,--gc-sections \
	-Wl,--major-os-version,4 -Wl,--minor-os-version,0 \
	-Wl,--major-subsystem-version,4 -Wl,--minor-subsystem-version,0 \
	-Wl,--image-base,0x7a800000 -Wl,--no-insert-timestamp \
	-Wl,--disable-dynamicbase -Wl,--disable-nxcompat

LIBS := -lkernel32 -luser32 -lgcc

SRCS := src/iefixup.c src/buffer.c src/forward.c src/html.c src/libc.c src/log.c src/stream.c src/utf8.c
OBJS := $(patsubst src/%.c,$(BUILD)/%.o,$(SRCS)) $(BUILD)/forward-table.o $(BUILD)/forward-stubs.o

all: $(BUILD)/WININET.DLL

$(BUILD)/WININET.DLL: $(OBJS) $(BUILD)/wininet.def
	$(CC) $(LDFLAGS) -o $(BUILD)/wininet-full.dll $(OBJS) $(BUILD)/wininet.def $(LIBS)
	$(STRIP) -o $@ $(BUILD)/wininet-full.dll

$(BUILD)/wininet.def $(BUILD)/forward-stubs.S $(BUILD)/forward-table.c &: tools/mkforwarders.py src/wininet.exports Makefile | $(BUILD)
	$(PYTHON) -I tools/mkforwarders.py src/wininet.exports $(BUILD)/wininet.def $(BUILD)/forward-stubs.S \
		$(BUILD)/forward-table.c $(HOOKS)

$(BUILD)/%.o: src/%.c src/*.h | $(BUILD)
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILD)/forward-table.o: $(BUILD)/forward-table.c src/forward.h
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILD)/forward-stubs.o: $(BUILD)/forward-stubs.S
	$(CC) $(CFLAGS) -c -o $@ $<

$(BUILD):
	mkdir -p $@

clean:
	rm -rf $(BUILD)

.PHONY: all clean
