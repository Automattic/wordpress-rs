#!/usr/bin/env python3
"""Drives the Kotlin example app on an Android emulator or device from a script of steps, over
adb, and saves a screenshot and a list of what's on screen wherever the script asks. It's how the
app's screens are checked without tapping by hand.

    scripts/example-app-adb-script.py <adb serial> <script> <output folder>

Install the app first (`./gradlew :example:androidApp:installDebug -PcargoTarget=arm64` from
`native/kotlin`). Keep the output folder outside the checkout when the screens show real content.

A script has one step a line. Blank lines and lines starting with `//` are skipped. An element is
named by its text or content description, by `#` and its test tag, or by `~` and part of its text.
A number and `:` in front picks which match, counting from 0: `2:~Actions for`.

    start                     Starts the app again.
    tap <element>             Scrolls down, then back up, until the element is on screen, and taps it.
    tapnow <element>          Taps the element where it is, without scrolling.
    tapany <a> | <b>          Taps the first of these that's on screen.
    type <element> => <text>  Taps a field and types into it.
    back                      Goes back a screen.
    down [n] / up [n]         Scrolls down or up, n times (once by default).
    wait <seconds>            Waits.
    waitfor <element> [s]     Waits until the element is on screen, for 30 seconds unless told otherwise.
    waitgone <element> [s]    Waits until the element is gone.
    expect <element>          Fails unless the element is on screen.
    shot <name>               Saves `<name>.png` and `<name>.txt`: the screen, and what's on it.
"""
import os
import pathlib, re, subprocess, sys, time
import xml.etree.ElementTree as ET

import shutil

if len(sys.argv) != 4:
    sys.exit(__doc__)
# `adb` from the PATH, or from the Android SDK.
sdk = os.environ.get('ANDROID_HOME') or str(pathlib.Path.home() / 'Library/Android/sdk')
ADB = shutil.which('adb') or str(pathlib.Path(sdk) / 'platform-tools/adb')
serial, script, out = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
out.mkdir(parents=True, exist_ok=True)

def adb(*args, binary=False):
    result = subprocess.run([ADB, '-s', serial, *args], capture_output=True, timeout=120)
    return result.stdout if binary else result.stdout.decode(errors='replace')

def nodes():
    for _ in range(4):
        adb('shell', 'uiautomator', 'dump', '/sdcard/ui.xml')
        xml = adb('exec-out', 'cat', '/sdcard/ui.xml')
        try:
            return list(ET.fromstring(xml).iter('node'))
        except ET.ParseError:
            time.sleep(1)
    raise SystemExit('The screen could not be read.')

def label(node):
    return node.get('text') or node.get('content-desc') or ''

def matches(element):
    index = None
    found = re.match(r'^(\d+):(.*)$', element)
    if found:
        index, element = int(found.group(1)), found.group(2)
    if element.startswith('#'):
        test = lambda node: node.get('resource-id') == element[1:]
    elif element.startswith('~'):
        test = lambda node: element[1:] in label(node)
    else:
        test = lambda node: label(node) == element
    hits = [node for node in nodes() if test(node)]
    if index is not None:
        hits = hits[index:index + 1]
    return hits

def center(node):
    left, top, right, bottom = map(int, re.findall(r'\d+', node.get('bounds')))
    return (left + right) // 2, (top + bottom) // 2

def swipe(up):
    y1, y2 = (1700, 800) if up else (800, 1700)
    adb('shell', 'input', 'swipe', '540', str(y1), '540', str(y2), '300')
    time.sleep(0.8)

def tap(element, scrolls):
    # Looks down the screen first, then back up it: the element may be above where the list is.
    for towards_the_end in ([True, False] if scrolls else [True]):
        for _ in range(14 if scrolls else 1):
            hits = matches(element)
            if hits:
                x, y = center(hits[0])
                adb('shell', 'input', 'tap', str(x), str(y))
                time.sleep(0.8)
                return
            if scrolls:
                swipe(up=towards_the_end)
    raise RuntimeError(f'{element} is not on screen')

def wait(element, seconds, present):
    deadline = time.time() + seconds
    while time.time() < deadline:
        if bool(matches(element)) == present:
            return
        time.sleep(1)
    raise RuntimeError(f'{element} was {"not there" if present else "still there"} after {seconds:g}s')

def shot(name):
    (out / f'{name}.png').write_bytes(adb('exec-out', 'screencap', '-p', binary=True))
    lines = []
    for node in nodes():
        text, tag = label(node), node.get('resource-id')
        if text or tag:
            lines.append(f'{text!r}' + (f' #{tag}' if tag else ''))
    (out / f'{name}.txt').write_text('\n'.join(lines) + '\n')

def run(step):
    command, _, argument = step.partition(' ')
    argument = argument.strip()
    if command == 'start':
        adb('shell', 'am', 'start', '-S', '-W', '-n', 'rs.wordpress.example/.ui.welcome.WelcomeActivity')
        time.sleep(2)
    elif command == 'tap': tap(argument, scrolls=True)
    elif command == 'tapnow': tap(argument, scrolls=False)
    elif command == 'tapany':
        # Taps the first of several elements that's on screen: `tapany A | B`.
        for choice in [part.strip() for part in argument.split('|')]:
            if matches(choice):
                tap(choice, scrolls=False)
                return
        raise RuntimeError(f'None of {argument} is on screen')
    elif command == 'type':
        element, _, text = argument.partition(' => ')
        tap(element.strip(), scrolls=True)
        adb('shell', 'input', 'text', text.strip().replace(' ', '%s'))
        time.sleep(0.8)
    elif command == 'back':
        adb('shell', 'input', 'keyevent', 'KEYCODE_BACK'); time.sleep(0.8)
    elif command in ('down', 'up'):
        for _ in range(int(argument or 1)): swipe(up=command == 'down')
    elif command == 'wait': time.sleep(float(argument or 1))
    elif command in ('waitfor', 'waitgone'):
        parts = argument.rsplit(' ', 1)
        seconds = float(parts[1]) if len(parts) == 2 and re.fullmatch(r'[\d.]+', parts[1]) else None
        wait(parts[0] if seconds is not None else argument, seconds or 30, present=command == 'waitfor')
    elif command == 'expect':
        if not matches(argument): raise RuntimeError(f'Expected {argument} on screen')
    elif command == 'shot': shot(argument)
    else: raise RuntimeError(f"Unknown step '{command}'")

transcript = []
status = 0
for number, line in enumerate(script.read_text().splitlines(), 1):
    step = line.strip()
    if not step or step.startswith('//'):
        continue
    try:
        run(step)
        transcript.append(f'ok    {number}: {step}')
    except Exception as error:
        transcript.append(f'FAIL  {number}: {step} — {error}')
        shot(f'failure-line-{number}')
        status = 1
        break
(out / 'transcript.txt').write_text('\n'.join(transcript) + '\n')
print('\n'.join(transcript))
sys.exit(status)
