#!/usr/bin/env python3
"""Compare serial release planning times against a saved binary from before the fix.

Example: python3 -I tools/check_leader_planning_time.py BEFORE AFTER PART.step ...
The aggregate median must improve by 20%; no absolute machine-specific time limit.
Run on an idle machine, with identical release profiles and recognition inputs.
"""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("parts", nargs="+")
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--max-ratio", type=float, default=0.8)
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    if args.runs < 1 or not 0 < args.max_ratio < 1:
        parser.error("runs must be positive and max-ratio between zero and one")
    results = {}
    for part in args.parts:
        times = {"before": [], "after": []}
        for run in range(args.runs):
            # Alternate order to avoid consistently favouring a warmer filesystem cache.
            for which in (["before", "after"] if run % 2 == 0 else ["after", "before"]):
                start = time.perf_counter()
                reply = subprocess.run([getattr(args, which), "plan", part],
                                       capture_output=True, check=True, timeout=600)
                times[which].append(time.perf_counter() - start)
                plan = json.loads(reply.stdout)
                assert "dimensions" in plan, plan
        medians = {key: statistics.median(value) for key, value in times.items()}
        results[Path(part).stem] = {**medians, "samples": times}
    ratio = sum(r["after"] for r in results.values()) / sum(r["before"] for r in results.values())
    report = {"parts": results, "aggregate_ratio": ratio, "max_ratio": args.max_ratio}
    if args.out:
        args.out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    assert ratio <= args.max_ratio, f"planning performance regressed: ratio {ratio:.3f} > {args.max_ratio}"


if __name__ == "__main__":
    main()
