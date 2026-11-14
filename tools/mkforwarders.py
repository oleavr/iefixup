import sys
from pathlib import Path


def main():
    exports_path, def_path, asm_path, c_path, *hooks = sys.argv[1:]
    exports = parse_exports(Path(exports_path))
    forwarded = [(ordinal, name) for ordinal, name in exports if name not in hooks]
    Path(def_path).write_text(render_def(exports))
    Path(asm_path).write_text(render_asm(forwarded))
    Path(c_path).write_text(render_c(forwarded))


def parse_exports(path):
    exports = []
    for line in path.read_text().splitlines():
        ordinal, name = line.split()
        exports.append((int(ordinal), name))
    return exports


def render_def(exports):
    lines = ["LIBRARY WININET.DLL", "EXPORTS"]
    lines += [f"  {name} @{ordinal}" for ordinal, name in exports]
    return "\n".join(lines) + "\n"


def render_asm(forwarded):
    lines = ["  .text"]
    for index, (_, name) in enumerate(forwarded):
        lines += [
            f"  .globl _{name}",
            f"_{name}:",
            f"  jmp *_iefixup_forward_targets+{4 * index}",
        ]
    return "\n".join(lines) + "\n"


def render_c(forwarded):
    names = "".join(f"  \"{name}\",\n" for _, name in forwarded)
    return (
        "#include \"forward.h\"\n"
        "\n"
        f"const char * const iefixup_forward_names[IEFIXUP_N_FORWARDS] =\n{{\n{names}}};\n"
        "\n"
        "void * iefixup_forward_targets[IEFIXUP_N_FORWARDS];\n"
        "\n"
        f"const unsigned int iefixup_n_forwards = {len(forwarded)};\n"
    ).replace("IEFIXUP_N_FORWARDS", str(len(forwarded)))


main()
