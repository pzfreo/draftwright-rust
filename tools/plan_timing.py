"""Plan timing for draftwright-rust (the alpha 1 performance gate, docs/alpha1.md): per part, the
median wall time of `draftwright plan --analysis` (layout recovery included, as every caller gets
it), from an Analysis made once. Optionally writes each plan's JSON, to compare two builds' plans
byte for byte (`cmp -r`).

The time includes starting the process and reading the Analysis (about 2 ms on NIST FTC 06); time
on a loaded machine runs high, so compare builds on the same machine at the same time.

Usage: python3 -I tools/plan_timing.py <draftwright binary> <work dir> [--runs N] [--plans]
       <step files...>
(the parts: tools/quality_parts.txt, paths relative to $DRAFTWRIGHT_CORPUS). The Analyses are
kept in <work dir>/analysis and reused; with --plans the plans go to <work dir>/plans."""
import os, statistics, subprocess, sys, time

args = sys.argv[1:]
runs = 5
if "--runs" in args:
    i = args.index("--runs")
    runs = int(args[i + 1])
    del args[i:i + 2]
plans = "--plans" in args
args = [a for a in args if a != "--plans"]
dw, work, files = args[0], args[1], args[2:]
os.makedirs(f"{work}/analysis", exist_ok=True)
if plans:
    os.makedirs(f"{work}/plans", exist_ok=True)

for f in files:
    name = os.path.basename(f).split(".")[0]
    analysis = f"{work}/analysis/{name}.json"
    if not os.path.exists(analysis):
        a = subprocess.run([dw, "analyse", f], capture_output=True, text=True, timeout=600)
        if a.returncode != 0:
            print(f"{name:40} analyse failed: {a.stdout.strip()[:120]}")
            continue
        with open(analysis, "w") as out:
            out.write(a.stdout)
    times = []
    for _ in range(runs):
        t = time.perf_counter()
        p = subprocess.run([dw, "plan", "--analysis", analysis], capture_output=True, timeout=600)
        times.append((time.perf_counter() - t) * 1000)
    if p.returncode != 0:
        print(f"{name:40} plan failed: {p.stdout.decode()[:120]}")
        continue
    if plans:
        with open(f"{work}/plans/{name}.json", "wb") as out:
            out.write(p.stdout)
    print(f"{name:40} {statistics.median(times):9.1f} ms")
