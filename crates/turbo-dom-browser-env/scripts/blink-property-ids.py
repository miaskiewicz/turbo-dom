#!/usr/bin/env python3
"""Print Blink's CSSPropertyID order for animations.js's BLINK_PROPERTIES (turbo-dom#233).

Usage: python3 scripts/blink-property-ids.py css_properties.json5

Pass third_party/blink/renderer/core/css/css_properties.json5 from the Chrome control's
version (156.0.8078.47). The order is css_properties.py's add_properties(): longhands, then
shorthands, each sorted by (-priority, internal-visited first, name without its leading
dash), numbered from 2. Aliases get no ID of their own. Output: the names, space-separated,
in ID order, so the ID of the name at index i is i + 2.
"""
import re
import sys


def entries(text):
    """The top-level objects of the json5 `data` array, as source text."""
    text = re.sub(r'(?m)^\s*//.*$', '', text)
    start = text.index('[', text.index('data:'))
    out, depth, begin, quote, i = [], 0, None, None, start + 1
    while i < len(text):
        c = text[i]
        if quote:
            if c == '\\':
                i += 1
            elif c == quote:
                quote = None
        elif c in '"\'':
            quote = c
        elif c == '{':
            if depth == 0:
                begin = i
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                out.append(text[begin:i + 1])
        elif c == ']' and depth == 0:
            break
        i += 1
    return out


def sorting_key(prop):
    name, priority = prop[0], prop[1]
    bare = name[1:] if name.startswith('-') else name
    visited = 0 if bare.startswith(('internal-visited-', 'internal-forced-visited-')) else 1
    return (-priority, visited, bare)


def main(path):
    props = []
    for entry in entries(open(path, encoding='utf-8').read()):
        name = re.search(r'\bname:\s*"([^"]+)"', entry).group(1)
        priority = re.search(r'\bpriority:\s*(-?\d+)', entry)
        if re.search(r'\balias_for:\s*"', entry):
            continue
        shorthand = bool(re.search(r'\blonghands:\s*\[', entry))
        props.append((name, int(priority.group(1)) if priority else 0, shorthand))
    longhands = sorted((p for p in props if not p[2]), key=sorting_key)
    shorthands = sorted((p for p in props if p[2]), key=sorting_key)
    print(' '.join(p[0] for p in longhands + shorthands))


if __name__ == '__main__':
    main(sys.argv[1])
