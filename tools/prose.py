#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
"""Check this repository's prose the way audit.sh checks its numbers.

Two kinds of check. Stock phrases, which are a defect at any density, and
distribution metrics against a baseline measured from this repository's own
earlier writing: 6.6 em-dashes per thousand words, 12% of sentences opening
with "The", a sentence-length standard deviation of 25.

The thresholds sit well outside that baseline on purpose. This is meant to
catch drafting that has drifted somewhere else, not to enforce a voice.

    python3 tools/prose.py            check every markdown file and lib.rs
    python3 tools/prose.py FILE...    check specific files
    python3 tools/prose.py --stats    print the measurements and exit 0
"""
import re, statistics, sys, pathlib

MAX_EMDASH_PER_1K = 15.0   # baseline 6.6
MAX_THE_OPENERS   = 30     # per cent; baseline 12
MIN_SENTENCE_SD   = 8.0    # baseline 25.1; below this the rhythm is machine-flat
MIN_WORDS         = 120    # shorter files are not a distribution

STOCK = [
    r"delve", r"leverag(?:e|ing)", r"seamless", r"robust solution", r"cutting-edge",
    r"state-of-the-art", r"game-?changer", r"paradigm shift", r"holistic",
    r"synerg(?:y|ies)", r"tapestry", r"myriad", r"plethora", r"multifaceted",
    r"unlock(?:ing)? the (?:power|potential)", r"empower(?:ing)?",
    r"it(?:'s| is) worth noting", r"it(?:'s| is) important to note",
    r"at its core", r"in the realm of", r"the key takeaway",
    r"let(?:'s| us) (?:dive|delve)", r"a testament to",
    r"plays? a (?:crucial|key|vital|pivotal) role", r"navigat\w+ the complexit",
    r"in today(?:'s)? \w+ world", r"ever-(?:evolving|changing)",
    r"first and foremost", r"needless to say", r"in conclusion", r"to sum up",
    r"not (?:just|merely|only) (?:a|an|about)[^.]{0,60}\bbut\b",
    r"^\s*(?:Moreover|Furthermore|Additionally|Notably|Importantly|Ultimately),",
    r"may potentially", r"could possibly",
]

def prose(text):
    out, fence = [], False
    for line in text.split("\n"):
        s = line.strip()
        if s.startswith("```"):
            fence = not fence
            continue
        if fence or not s or s.startswith(("|", "#", "<", ">", "[!", "---", "    ")):
            continue
        s = re.sub(r"`[^`]*`", "CODE", s)
        s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", s)
        # A list item's "term — definition" dash is layout rather than prose.
        if re.match(r"^\s*[-*]\s+", s):
            s = re.sub(r"^\s*[-*]\s+", "", s).replace("\u2014", "", 1)
        s = re.sub(r"^\s*///?!?\s?", "", s)
        out.append(s)
    return " ".join(out)

def sentences(t):
    return [s.strip() for s in re.split(r'(?<=[.!?])\s+(?=[A-Z"\u201c])', t)
            if len(s.split()) > 2]

def check(path, stats_only=False):
    raw = path.read_text(encoding="utf-8", errors="ignore")
    fails = []

    fence = False
    for n, line in enumerate(raw.split("\n"), 1):
        if line.strip().startswith("```"):
            fence = not fence
            continue
        if fence:
            continue
        for pat in STOCK:
            m = re.search(pat, line, re.I | re.M)
            if m:
                fails.append(f"{path}:{n}  stock phrase {m.group(0).strip()!r}")

    t = prose(raw)
    words = t.split()
    ss = sentences(t)
    if len(words) < MIN_WORDS or len(ss) < 6:
        return fails, None

    L = [len(s.split()) for s in ss]
    m = dict(
        words=len(words),
        emdash=round(t.count("\u2014") * 1000 / len(words), 1),
        the=round(sum(1 for s in ss if s.startswith("The ")) * 100 / len(ss)),
        sd=round(statistics.pstdev(L), 1),
        mean=round(statistics.mean(L), 1),
    )
    if not stats_only:
        if m["emdash"] > MAX_EMDASH_PER_1K:
            fails.append(f"{path}  {m['emdash']} em-dashes per 1000 words, limit {MAX_EMDASH_PER_1K}")
        if m["the"] > MAX_THE_OPENERS:
            fails.append(f"{path}  {m['the']}% of sentences open with \"The\", limit {MAX_THE_OPENERS}%")
        if m["sd"] < MIN_SENTENCE_SD:
            fails.append(f"{path}  sentence-length sd {m['sd']}, floor {MIN_SENTENCE_SD} — the rhythm is flat")
    return fails, m

def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    stats = "--stats" in sys.argv
    root = pathlib.Path(".")
    targets = [pathlib.Path(a) for a in args] or (
        sorted(p for p in root.rglob("*.md")
               if ".git" not in p.parts and "target" not in p.parts)
        + [root / "src" / "lib.rs"])

    allfails, rows = [], []
    for p in targets:
        if not p.exists():
            continue
        f, m = check(p, stats)
        allfails += f
        if m:
            rows.append((p, m))

    if stats:
        print(f"{'file':<34}{'words':>7}{'em-dash/1k':>12}{'The %':>7}{'sd':>7}{'mean':>7}")
        for p, m in rows:
            print(f"{str(p):<34}{m['words']:>7}{m['emdash']:>12}{m['the']:>7}{m['sd']:>7}{m['mean']:>7}")
        return 0

    for f in allfails:
        print("  " + f)
    print(f"  {len(rows)} file(s) measured, {len(allfails)} failure(s)")
    return 1 if allfails else 0

if __name__ == "__main__":
    sys.exit(main())
