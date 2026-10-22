"""Makes dompurify.json from DOMPurify's test cases, for `reader::safety_tests`.

    curl -fLO https://raw.githubusercontent.com/cure53/DOMPurify/<commit>/test/fixtures/expect.mjs
    python3 -I import-dompurify-cases.py expect.mjs <commit> dompurify.json

expect.mjs is a JavaScript module, so this reads it as data rather than running it. It fails on
anything that isn't a literal: strings, arrays, objects, `+` between strings, comments and trailing
commas."""
import json
import sys

src = open(sys.argv[1], encoding="utf-8").read()
prefix = "export default"
start = src.index(prefix) + len(prefix)
i = start

def fail(message):
    raise SystemExit(f"{message} at offset {i}")

def skip():
    global i
    while i < len(src):
        if src[i].isspace():
            i += 1
        elif src.startswith("//", i):
            i = src.index("\n", i)
        elif src.startswith("/*", i):
            i = src.index("*/", i) + 2
        else:
            break

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "b": "\b", "f": "\f", "v": "\v", "0": "\0"}

def string():
    global i
    quote = src[i]
    i += 1
    out = []
    while True:
        c = src[i]
        if c == quote:
            i += 1
            return "".join(out)
        if quote == "`" and src.startswith("${", i):
            fail("template substitution")
        if c == "\\":
            n = src[i + 1]
            if n == "u":
                if src[i + 2] == "{":
                    end = src.index("}", i)
                    out.append(chr(int(src[i + 3:end], 16)))
                    i = end + 1
                else:
                    out.append(chr(int(src[i + 2:i + 6], 16)))
                    i += 6
                continue
            if n == "x":
                out.append(chr(int(src[i + 2:i + 4], 16)))
                i += 4
                continue
            if n == "\n":
                i += 2
                continue
            if n == "0" and src[i + 2].isdigit():
                fail("octal escape")
            out.append(ESCAPES.get(n, n))
            i += 2
            continue
        if c == "\n" and quote != "`":
            fail("newline in string")
        out.append(c)
        i += 1

def value():
    global i
    skip()
    c = src[i]
    if c in "'\"`":
        s = string()
        skip()
        while src[i] == "+":
            i += 1
            skip()
            if src[i] not in "'\"`":
                fail("concatenation with a non-string")
            s += string()
            skip()
        return s
    if c == "[":
        i += 1
        items = []
        while True:
            skip()
            if src[i] == "]":
                i += 1
                return items
            items.append(value())
            skip()
            if src[i] == ",":
                i += 1
            elif src[i] != "]":
                fail("expected , or ]")
    if c == "{":
        i += 1
        obj = {}
        while True:
            skip()
            if src[i] == "}":
                i += 1
                return obj
            if src[i] in "'\"":
                key = string()
            else:
                j = i
                while src[i].isalnum() or src[i] == "_":
                    i += 1
                key = src[j:i]
                if not key:
                    fail("expected a key")
            skip()
            if src[i] != ":":
                fail("expected :")
            i += 1
            obj[key] = value()
            skip()
            if src[i] == ",":
                i += 1
            elif src[i] != "}":
                fail("expected , or }")
    fail(f"not a literal ({c!r})")

cases = value()
skip()
if src[i:].strip() not in (";", ""):
    fail("trailing content")

keys = {}
for case in cases:
    for key in case:
        keys[key] = keys.get(key, 0) + 1
print(f"{len(cases)} cases; keys: {keys}", file=sys.stderr)

commit = sys.argv[2]
out = {
    "source": f"https://github.com/cure53/DOMPurify/blob/{commit}/test/fixtures/expect.mjs",
    "license": "DOMPurify is MPL-2.0 OR Apache-2.0. These cases are used under MPL-2.0, like the rest of wordpress-rs.",
    "note": "Only each case's title and payload. DOMPurify's expected output is DOMPurify's own, so the test checks the cleanup's output for safety instead.",
    "cases": [{"title": case.get("title", ""), "payload": case["payload"]} for case in cases],
}
with open(sys.argv[3], "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=1)
    f.write("\n")
