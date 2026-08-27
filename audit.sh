#!/usr/bin/env bash
# audit.sh — total audit of the dy-wcet repository.
#
# Checks every number this repository states in prose against the source that
# is supposed to back it, then the invariants the crate claims for itself.
#
# SPDX-License-Identifier: Apache-2.0 OR MIT
# Copyright (c) 2026 Denis Yermakou — DY Research
#
#   ./audit.sh              audit the repository in the current directory
#   ./audit.sh /path/repo   audit somewhere else
#   ./audit.sh --no-cargo   skip the toolchain gates (phone, no Rust)
#   ./audit.sh --net        also resolve external URLs
#
# Exit code is the number of FAILs, so this drops into CI unchanged.

REPO="."
RUN_CARGO=1
RUN_NET=0
PRERELEASE=0
for a in "$@"; do
  case "$a" in
    --no-cargo) RUN_CARGO=0 ;;
    --pre-release) PRERELEASE=1 ;;
    --net)      RUN_NET=1 ;;
    -h|--help)  sed -n '2,18p' "$0"; exit 0 ;;
    *)          REPO="$a" ;;
  esac
done
cd "$REPO" 2>/dev/null || { echo "no such directory: $REPO"; exit 1; }

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  R=$'\033[31m'; G=$'\033[32m'; Y=$'\033[33m'; B=$'\033[34m'; D=$'\033[2m'; Z=$'\033[0m'
else
  R=; G=; Y=; B=; D=; Z=
fi

PASS=0; WARN=0; FAIL=0
pass() { PASS=$((PASS+1)); printf '  %sPASS%s  %s\n' "$G" "$Z" "$1"; }
warn() { WARN=$((WARN+1)); printf '  %sWARN%s  %s\n' "$Y" "$Z" "$1"; }
fail() { FAIL=$((FAIL+1)); printf '  %sFAIL%s  %s\n' "$R" "$Z" "$1"; }
note() { printf '        %s%s%s\n' "$D" "$1" "$Z"; }
sect() { printf '\n%s%s%s\n' "$B" "$1" "$Z"; }

# Termux has no /tmp. $TMPDIR is set there and points inside $PREFIX; falling
# back to a directory beside the repo keeps this runnable on a phone.
SCRATCH=$(mktemp -d 2>/dev/null) || SCRATCH=""
if [ -z "$SCRATCH" ] || [ ! -w "$SCRATCH" ]; then
  SCRATCH="./.dy-audit-scratch.$$"
  mkdir -p "$SCRATCH" 2>/dev/null || { echo "no writable scratch directory"; exit 1; }
fi
trap 'rm -rf "$SCRATCH"' EXIT INT TERM

# grep -c prints 0 AND exits 1 on no match, so `|| echo 0` would print two zeros
cnt() { n=$(grep -cE "$1" "$2" 2>/dev/null | head -1); echo "${n:-0}"; }
cntf() { n=$(grep -cF "$1" "$2" 2>/dev/null | head -1); echo "${n:-0}"; }

numword() {
  case "$1" in
    0) echo zero;; 1) echo one;; 2) echo two;; 3) echo three;; 4) echo four;;
    5) echo five;; 6) echo six;; 7) echo seven;; 8) echo eight;; 9) echo nine;;
    10) echo ten;; 11) echo eleven;; 12) echo twelve;; 13) echo thirteen;;
    14) echo fourteen;; 15) echo fifteen;; 16) echo sixteen;;
    *) echo "$1";;
  esac
}

printf '%s\n' "════════════════════════════════════════════════════════════"
printf '  dy-wcet — total audit\n'
printf '  %s\n' "$(pwd)"
printf '  %s\n' "$(date -u '+%Y-%m-%d %H:%M UTC')"
printf '%s\n' "════════════════════════════════════════════════════════════"

# ─────────────────────────────────────────────────────────── A. inventory
sect "A · Inventory"

for f in Cargo.toml src/lib.rs tests/on_paper.rs README.md .github/workflows/ci.yml; do
  [ -f "$f" ] && pass "$f" || fail "$f is missing"
done
for f in AUDIT.md BOUNTY.md CHANGELOG.md CONTRIBUTING.md SECURITY.md CITATION.cff; do
  [ -f "$f" ] && pass "$f" || warn "$f is missing"
done
for f in LICENSE-APACHE LICENSE-MIT; do
  if [ -f "$f" ]; then pass "$f"
  else
    fail "$f is missing while Cargo.toml declares a dual licence"
    note "cargo publish ships no licence text; 'Apache-2.0 OR MIT' names files that are not here"
  fi
done
[ -f rust-toolchain.toml ] && pass "rust-toolchain.toml pins the toolchain" \
  || note "no rust-toolchain.toml — MSRV is stated in Cargo.toml only"

# ─────────────────────────────────────────── B. stated numbers vs source
sect "B · Every stated number, against the source that backs it"

UNIT=$(awk '/^mod tests|^#\[cfg\(test\)\]/,0' src/lib.rs 2>/dev/null | grep -c '#\[test\]' | head -1)
UNIT=${UNIT:-0}
INTEG=$(cnt '#\[test\]' tests/on_paper.rs)
OTHER=0
for f in tests/*.rs; do
  case "$f" in tests/on_paper.rs|"tests/*.rs") continue ;; esac
  OTHER=$(( OTHER + $(cnt '#\[test\]' "$f") ))
done
TOTAL=$((UNIT+INTEG+OTHER))
note "counted: $UNIT unit · $INTEG on-paper · $OTHER other integration · $TOTAL total"

for pair in "$UNIT unit" "$INTEG integration"; do
  set -- $pair; n=$1; kind=$2
  w=$(numword "$n")
  if grep -qiE "\b($w|$n)\b[^.]{0,20}$kind tests" README.md 2>/dev/null; then
    pass "README states $kind tests as $w — matches source"
  else
    claimed=$(grep -oiE '\b[a-z]+\b +'"$kind"' tests' README.md 2>/dev/null | head -1)
    fail "README '$kind tests' count does not match source ($n)"
    [ -n "$claimed" ] && note "README says: $claimed"
  fi
done

BADGE=$(grep -oE 'tests-[0-9]+' README.md 2>/dev/null | head -1 | cut -d- -f2)
if [ -n "$BADGE" ]; then
  [ "$BADGE" = "$TOTAL" ] && pass "tests badge says $BADGE, source has $TOTAL" \
    || fail "tests badge says $BADGE, source has $TOTAL"
else
  note "no tests badge in README"
fi

MAXT=$(grep -oE 'MAX_TASKS: usize = [0-9]+' src/lib.rs 2>/dev/null | grep -oE '[0-9]+$')
if [ -n "$MAXT" ]; then
  w=$(numword "$MAXT")
  grep -qiE "\b($w|$MAXT)\b tasks maximum|maximum.{0,12}\b($w|$MAXT)\b tasks" README.md \
    && pass "MAX_TASKS = $MAXT and README agrees" \
    || warn "MAX_TASKS = $MAXT — could not confirm README states the same"
fi

DEPS=$(awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f && NF && $0 !~ /^#/' Cargo.toml | wc -l | tr -d ' ')
DEPBADGE=$(grep -oE 'dependencies-[0-9]+' README.md 2>/dev/null | head -1 | cut -d- -f2)
if [ -n "$DEPBADGE" ]; then
  [ "$DEPS" = "$DEPBADGE" ] && pass "dependencies badge says $DEPBADGE, Cargo.toml has $DEPS" \
    || fail "dependencies badge says $DEPBADGE, Cargo.toml has $DEPS"
fi

CVER=$(grep -m1 -oE '^version *= *"[^"]+"' Cargo.toml | cut -d'"' -f2)
CHVER=$(grep -m1 -oE '^## \[[0-9][^]]*\]' CHANGELOG.md 2>/dev/null | tr -d '#[] ')
CITVER=$(grep -m1 -oE '^version: *"[^"]+"' CITATION.cff 2>/dev/null | cut -d'"' -f2)
note "Cargo.toml $CVER · CHANGELOG $CHVER · CITATION.cff $CITVER"
[ "$CVER" = "$CHVER" ] && pass "Cargo.toml version matches the newest CHANGELOG entry" \
  || fail "Cargo.toml $CVER but newest CHANGELOG entry is $CHVER"
if [ -n "$CITVER" ]; then
  [ "$CVER" = "$CITVER" ] && pass "CITATION.cff version matches Cargo.toml" \
    || fail "CITATION.cff says $CITVER, Cargo.toml says $CVER"
fi

CHDATE=$(grep -m1 -oE '^## \['"$CHVER"'\][^0-9]*[0-9]{4}-[0-9]{2}-[0-9]{2}' CHANGELOG.md 2>/dev/null | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}')
CITDATE=$(grep -m1 -oE 'date-released: *"?[0-9]{4}-[0-9]{2}-[0-9]{2}' CITATION.cff 2>/dev/null | grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}')
if [ -n "$CHDATE" ] && [ -n "$CITDATE" ]; then
  [ "$CHDATE" = "$CITDATE" ] && pass "CITATION.cff release date matches CHANGELOG ($CHDATE)" \
    || fail "CITATION.cff date-released $CITDATE, CHANGELOG dates $CHVER as $CHDATE"
fi

README_DEP=$(grep -m1 -oE 'dy-wcet *= *"[^"]+"' README.md 2>/dev/null | cut -d'"' -f2)
if [ -n "$README_DEP" ]; then
  case "$CVER" in
    "$README_DEP"*) pass "README install line \"$README_DEP\" is satisfied by $CVER" ;;
    *)              fail "README install line \"$README_DEP\" does not cover $CVER" ;;
  esac
fi

# ───────────────────────────────────── C. the invariants the crate claims
sect "C · The invariants the crate claims for itself"

if grep -nE '\bf32\b|\bf64\b' src/lib.rs | grep -v '^\s*[0-9]*:\s*//' | grep -qv 'in `f64`'; then
  fail "floating point in a crate whose whole point is integer arithmetic"
  grep -nE '\bf32\b|\bf64\b' src/lib.rs | grep -v 'in `f64`' | sed 's/^/        /'
else
  pass "no floating point in src/lib.rs"
fi

grep -q '#!\[forbid(unsafe_code)\]' src/lib.rs \
  && pass "#![forbid(unsafe_code)] present" || fail "unsafe is not forbidden"
grep -q 'no_std' src/lib.rs \
  && pass "no_std attribute present" || fail "no_std claimed in README but not in src"
grep -q '#!\[deny(missing_docs)\]' src/lib.rs \
  && pass "#![deny(missing_docs)] present" || warn "missing_docs is not denied"

for bad in 'wrapping_add' 'wrapping_mul' 'saturating_add' 'saturating_mul'; do
  n=$(cntf "$bad" src/lib.rs)
  [ "$n" -eq 0 ] && pass "no $bad" || fail "$n use(s) of $bad — the one direction an error must not go"
done
CHK=$(cnt 'checked_(add|mul|div|sub)' src/lib.rs)
note "checked arithmetic call sites: $CHK"

UNSCH=$(cntf 'Response::Unbounded(' src/lib.rs)
note "Response::Unschedulable is returned from $UNSCH distinct sites"
# The doc for Unschedulable must enumerate its causes, not gesture at them.
# Until 0.1.2 it named two of four, and no test could have caught that.
DOCCAUSES=$(awk '/^pub enum Unbounded/,/^}/' src/lib.rs 2>/dev/null | grep -cE '^\s*[A-Z][A-Za-z]*[,(]' | head -1)
DOCCAUSES=${DOCCAUSES:-0}
note "Unbounded distinguishes $DOCCAUSES reason(s); Unbounded is constructed at $UNSCH sites"
if [ "$DOCCAUSES" -ge 4 ]; then
  pass "an unbounded answer names which of $DOCCAUSES things went wrong"
else
  fail "Unbounded carries only $DOCCAUSES reason(s)"
  grep -nE 'return Response::Unschedulable|^\s*Response::Unschedulable\s*$' src/lib.rs | sed 's/^/        /'
  note "a cause that is real but undocumented is the defect class this repo exists for"
fi

# every paper test carries its derivation — the CI rule, run locally
if [ -f tests/on_paper.rs ] && command -v python3 >/dev/null 2>&1; then
  if python3 - <<'PY'
import re, sys, pathlib
t = pathlib.Path("tests/on_paper.rs").read_text(encoding="utf-8")
fns = re.findall(r"((?:///[^\n]*\n|\s*)*)#\[test\]\s*\nfn (\w+)", t)
bad = [n for doc, n in fns if "→" not in doc and "R = " not in doc and "No derivation" not in doc]
if bad:
    print("        " + ", ".join(bad)); sys.exit(1)
sys.exit(0)
PY
  then pass "every paper test carries its derivation"
  else fail "paper tests asserting a value without deriving it (listed above)"
  fi
fi

# ──────────────────────────────────────────────────────────── D. links
sect "D · Links"

MD=$(ls -1 *.md docs/*.md case-studies/*.md 2>/dev/null)
for f in $MD; do
  # relative file links
  for tgt in $(grep -oE '\]\(([A-Za-z0-9_./-]+\.(md|rs|toml|json|yml))' "$f" 2>/dev/null | sed 's/^](//'); do
    base=$(dirname "$f"); [ "$base" = "." ] && cand="$tgt" || cand="$base/$tgt"
    if [ -e "$cand" ] || [ -e "$tgt" ]; then :; else fail "$f → $tgt does not exist"; fi
  done
done
pass "relative file links checked across $(echo "$MD" | wc -w | tr -d ' ') markdown files"

if command -v python3 >/dev/null 2>&1; then
  python3 - <<'PY'
import re, pathlib, sys
bad=0
for p in list(pathlib.Path('.').glob('*.md'))+list(pathlib.Path('docs').glob('*.md')):
    md=p.read_text(encoding='utf-8',errors='ignore')
    heads=re.findall(r'^#{1,6} (.+)$', md, re.M)
    def slug(h):
        s=re.sub(r'\[|\]|\(.*?\)','',h).lower()
        s=re.sub(r'[^\w\s-]','',s)
        return re.sub(r'\s+','-',s.strip())
    have={slug(h) for h in heads}
    for a in dict.fromkeys(re.findall(r'\]\(#([a-z0-9\-]+)\)', md)):
        if a not in have:
            print(f"        {p} → #{a} resolves to no heading"); bad+=1
sys.exit(1 if bad else 0)
PY
  [ $? -eq 0 ] && pass "every internal anchor resolves to a heading" \
    || fail "internal anchors above point at nothing"
fi

if [ "$RUN_NET" = "1" ] && command -v curl >/dev/null 2>&1; then
  for u in $(grep -ohE 'https?://[A-Za-z0-9./_%#?=&:-]+' $MD 2>/dev/null \
             | sed 's/[.,)]$//' | sort -u | head -40); do
    code=$(curl -s -o /dev/null -L --max-time 12 -w '%{http_code}' "$u" 2>/dev/null)
    case "$code" in
      2*|3*) : ;;
      000)   warn "unreachable: $u" ;;
      *)     warn "HTTP $code: $u" ;;
    esac
  done
  pass "external URLs resolved"
else
  note "external URL check skipped (pass --net to run it)"
fi

# ─────────────────────────────────────────────────── D2. prose
sect "D2 · Prose"

if [ -f tools/prose.py ] && command -v python3 >/dev/null 2>&1; then
  if python3 tools/prose.py >"$SCRATCH/prose" 2>&1; then
    pass "$(tail -1 "$SCRATCH/prose" | sed 's/^ *//')"
  else
    fail "prose checks"
    sed 's/^/      /' "$SCRATCH/prose"
  fi
else
  note "tools/prose.py not present"
fi

# ────────────────────────────────────────────── E. markdown hygiene
sect "E · Markdown hygiene"

for f in $MD; do
  h1=$(cnt '^# ' "$f")
  [ "$h1" -le 1 ] || fail "$f has $h1 level-1 headings"
  # A jump is a defect, with one carve-out: "# Title" followed immediately by
  # "### Subtitle" is a deliberate typographic pattern, not a skipped level.
  awk -v F="$f" '
    /^#+ / { n++; lvl=length($1)
             sub_ok = (n==2 && prev==1 && lvl==3)
             if (prev && lvl>prev+1 && !sub_ok)
               printf "        %s:%d heading jumps h%d → h%d\n", F, NR, prev, lvl
             prev=lvl }
  ' "$f"
  grep -qP '\t' "$f" 2>/dev/null && warn "$f contains tabs"
  grep -qE ' +$' "$f" 2>/dev/null && warn "$f has trailing whitespace"
  grep -q $'\r' "$f" 2>/dev/null && warn "$f has CRLF line endings"
  long=$(awk 'length>100 && $0 !~ /^\|/ && $0 !~ /http/ {n++} END{print n+0}' "$f")
  [ "$long" -gt 0 ] && note "$f: $long line(s) over 100 chars outside tables and URLs"
done
pass "heading levels, tabs, trailing whitespace and line endings checked"

# ─────────────────────────────────────────────────── F. SPDX headers
sect "F · Licensing and provenance"

for f in src/lib.rs tests/on_paper.rs; do
  grep -q 'SPDX-License-Identifier' "$f" 2>/dev/null \
    && pass "$f carries an SPDX header" || warn "$f has no SPDX header"
done
DECL=$(grep -m1 -oE '^license *= *"[^"]+"' Cargo.toml | cut -d'"' -f2)
note "Cargo.toml declares: $DECL"

# ─────────────────────────────────────────────── G. toolchain gates
sect "G · Toolchain gates"

if [ "$RUN_CARGO" = "1" ] && command -v cargo >/dev/null 2>&1; then
  run() { printf '  %s···%s  %s\n' "$D" "$Z" "$2"
          if eval "$1" >"$SCRATCH/gate" 2>&1; then pass "$2"
          else fail "$2"; tail -12 "$SCRATCH/gate" | sed 's/^/        /'; fi; }
  run "cargo fmt --all -- --check"                     "cargo fmt"
  run "cargo clippy --all-targets -- -D warnings"      "cargo clippy, warnings denied"
  run "cargo test"                                     "cargo test"
  run "RUSTDOCFLAGS='-D warnings' cargo doc --no-deps" "cargo doc, warnings denied"
  if rustup target list --installed 2>/dev/null | grep -q thumbv7em-none-eabihf; then
    run "cargo build --release --target thumbv7em-none-eabihf" "no_std build for thumbv7em-none-eabihf"
  else
    warn "thumbv7em-none-eabihf not installed — the no_std claim is unverified here"
    note "rustup target add thumbv7em-none-eabihf"
  fi
  run "cargo package --list --allow-dirty" "cargo package builds a file list"
  if cargo package --list --allow-dirty 2>/dev/null | grep -qi '^LICENSE'; then
    pass "the published archive would contain licence text"
  else
    fail "the published archive would contain no licence text"
  fi
else
  note "cargo not run — CI covers these gates on every push"
fi

# ───────────────────────────────────────────────── H. git hygiene
sect "H · Git"

if [ -d .git ]; then
  # Before a release commit these two cannot pass: the files were written a
  # moment ago and the tag goes on afterwards. A check that cannot pass at the
  # point it runs is noise, not a warning, so the release path says which
  # phase it is in and this reports accordingly.
  dirty=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
  if [ "$dirty" -eq 0 ]; then
    pass "working tree is clean"
  elif [ "$PRERELEASE" = "1" ]; then
    note "$dirty file(s) staged for the release commit, as expected at this point"
  else
    warn "$dirty uncommitted change(s)"
  fi
  # A shallow clone carries no tags. Reporting "no tags" from one is a false
  # finding, and this checker produced exactly that on 2026-08-27.
  SHALLOW=$(git rev-parse --is-shallow-repository 2>/dev/null)
  tag=$(git describe --tags --abbrev=0 2>/dev/null)
  if [ -n "$tag" ]; then
    if [ "${tag#v}" = "$CVER" ]; then
      pass "newest tag $tag matches Cargo.toml $CVER"
    elif [ "$PRERELEASE" = "1" ]; then
      note "newest tag $tag, manifest $CVER — v$CVER goes on after the commit"
    else
      warn "newest tag $tag, Cargo.toml $CVER"
    fi
  elif [ "$SHALLOW" = "true" ]; then
    note "shallow clone: tags are absent here, which says nothing about the remote"
    note "git fetch --tags --unshallow to check"
  else
    warn "no tags — BOUNTY.md judges claims against 'the latest published tag'"
  fi
  grep -q 'Cargo.lock' .gitignore 2>/dev/null \
    && note ".gitignore excludes Cargo.lock — correct for a library" || true
else
  note "not a git working tree"
fi

# ───────────────────────────────────────────────────────── summary
printf '\n%s\n' "════════════════════════════════════════════════════════════"
printf '  %sPASS %d%s   %sWARN %d%s   %sFAIL %d%s\n' "$G" "$PASS" "$Z" "$Y" "$WARN" "$Z" "$R" "$FAIL" "$Z"
printf '%s\n' "════════════════════════════════════════════════════════════"
exit "$FAIL"
