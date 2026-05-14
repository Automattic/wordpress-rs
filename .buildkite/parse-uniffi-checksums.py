#!/usr/bin/env python3
"""
Parses UniFFI checksum return values from llvm-objdump disassembly output.

Each checksum function is a tiny stub that loads an immediate into a register
and returns. This script extracts the function name and return value.

Supported architectures:
  - AArch64:   mov w0, #0x1234 ; ret
  - ARM Thumb: movw/mov.w/movs r0, #0x1234 ; bx lr
  - x86/x64:   movw $0x1234, %ax ; retl/retq

Usage: llvm-objdump -d ... | python3 parse-uniffi-checksums.py
   or: python3 parse-uniffi-checksums.py < disassembly.txt
"""

import re
import sys


def main():
    current_fn = None
    for line in sys.stdin:
        # Match function labels: <uniffi_wp_api_checksum_func_foo>:
        fn_match = re.search(r"<_?(uniffi_\w*checksum_\w+)>:", line)
        if fn_match:
            current_fn = fn_match.group(1)
            continue
        if current_fn:
            # ARM/AArch64: #0xNNNN (covers mov, movw, mov.w, movs with # prefix)
            m = re.search(r"#(0x[0-9a-fA-F]+)", line)
            if not m:
                # x86 AT&T syntax: movw $0x1234, %ax  (also movl, movq)
                m = re.search(r"\$(0x[0-9a-fA-F]+)", line)
            if not m:
                # x86 Intel syntax: mov eax, 0x1234
                m = re.search(r"mov\w*\s+\w+,\s*(0x[0-9a-fA-F]+)", line)
            if m:
                val = int(m.group(1), 16)
                print(f"{current_fn} {val}")
                current_fn = None


if __name__ == "__main__":
    main()
