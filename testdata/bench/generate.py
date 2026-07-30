#!/usr/bin/env python3
"""Generates the synthetic dataset used by the regex-caching benchmark.

The records are entirely made up — the point is volume and a `name`/`tag`
shape that the `$filter` conditions in `_config.json` can match against.

Usage:
    python3 testdata/bench/generate.py [record_count] [output_dir]

Defaults to 200000 records in testdata/bench/data, which is the size the
figures in README.md were measured at.
"""

import json
import os
import random
import sys

TAGS = ("alpha", "beta", "gamma")


def main() -> None:
    count = int(sys.argv[1]) if len(sys.argv) > 1 else 200_000
    out_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), "data")

    # Fixed seed so repeated runs compare like with like.
    random.seed(7)
    records = [
        {"id": i, "name": f"item-{i:06d}", "tag": random.choice(TAGS)} for i in range(count)
    ]

    os.makedirs(out_dir, exist_ok=True)
    target = os.path.join(out_dir, "items.json")
    with open(target, "w", encoding="utf-8") as out:
        json.dump(records, out)

    size_mb = os.path.getsize(target) / 1024 / 1024
    print(f"wrote {count} records to {target} ({size_mb:.1f} MiB)")


if __name__ == "__main__":
    main()
