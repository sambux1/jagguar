#!/usr/bin/env python3
"""Run the SHPRG memory bench and plot peak-heap overhead over output size."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt

REPO_ROOT = Path(__file__).resolve().parents[2]

plt.rcParams.update(
    {
        "font.family": "sans-serif",
        "text.usetex": True,
    }
)

RUN_RE = re.compile(
    r"--- (?P<label>.+?) \(target (?P<target>\d+) bytes of randomness, "
    r"(?P<elements>\d+) elements\) ---"
)
PEAK_RE = re.compile(r"At t-gmax:\s+([\d,]+) bytes")
STORAGE_BYTES_PER_ELEMENT = 16

# Bench labels -> plot labels (preserve bench order).
LABEL_DISPLAY = {
    "100_KiB": "100KiB",
    "1_MiB": "1MiB",
    "10_MiB": "10MiB",
}


@dataclass(frozen=True)
class BenchResult:
    label: str
    target_bytes: int
    elements: int
    peak_bytes: int

    @property
    def storage_bytes(self) -> int:
        return self.elements * STORAGE_BYTES_PER_ELEMENT

    @property
    def overhead(self) -> float:
        # Peak heap vs the labeled randomness volume (100KiB / 1MiB / 10MiB).
        return self.peak_bytes / self.target_bytes


def run_memory_bench() -> str:
    result = subprocess.run(
        ["cargo", "bench", "--bench", "shprg_memory"],
        cwd=REPO_ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    return result.stdout + result.stderr


def parse_results(output: str) -> list[BenchResult]:
    results: list[BenchResult] = []
    blocks = output.split("--- ")
    for block in blocks[1:]:
        run_match = RUN_RE.search("--- " + block)
        peak_match = PEAK_RE.search(block)
        if not run_match or not peak_match:
            continue

        bench_label = run_match.group("label")
        display = LABEL_DISPLAY.get(bench_label, bench_label.replace("_", ""))
        results.append(
            BenchResult(
                label=display,
                target_bytes=int(run_match.group("target")),
                elements=int(run_match.group("elements")),
                peak_bytes=int(peak_match.group(1).replace(",", "")),
            )
        )

    if not results:
        raise RuntimeError(
            "Could not parse any benchmark results from shprg_memory output.\n"
            + output[-4000:]
        )
    return results


def print_breakdown(results: list[BenchResult]) -> None:
    print("overhead = dhat peak heap / target randomness bytes\n")
    for r in results:
        print(
            f"{r.label}: peak={r.peak_bytes:,} B, "
            f"target={r.target_bytes:,} B, "
            f"storage={r.storage_bytes:,} B ({r.elements:,} × 16 B) "
            f"→ {r.overhead:.3f}x"
        )


def plot_overhead(results: list[BenchResult], output_path: Path, show: bool) -> None:
    labels = [r.label for r in results]
    factors = [r.overhead for r in results]

    fig, ax = plt.subplots(figsize=(5, 3.2))
    bars = ax.bar(labels, factors, color="#4c72b0", width=0.45, zorder=3)

    ymax = max(factors) * 1.18
    ax.set_ylim(1.0, ymax)

    ax.yaxis.grid(True, linestyle=":", linewidth=0.6, color="#cccccc", zorder=0)
    ax.set_axisbelow(True)

    for bar, factor in zip(bars, factors):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height() + (ymax - 1.0) * 0.02,
            f"{factor:.2f}x",
            ha="center",
            va="bottom",
            color="#222222",
        )

    ax.set_ylabel("Overhead Over Output Size")
    ax.tick_params(bottom=False)
    for spine in ("top", "right"):
        ax.spines[spine].set_visible(False)
    ax.spines["bottom"].set_color("#bbbbbb")
    ax.spines["left"].set_color("#bbbbbb")

    fig.tight_layout()
    fig.savefig(output_path, dpi=150, bbox_inches="tight")
    print(f"Wrote {output_path}")

    if show:
        plt.show()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=REPO_ROOT / "scripts/plotting/shprg_memory_overhead.png",
        help="Path for the output plot (default: scripts/plotting/shprg_memory_overhead.png)",
    )
    parser.add_argument(
        "--from-log",
        type=Path,
        help="Parse an existing bench log instead of running cargo bench",
    )
    parser.add_argument(
        "--no-show",
        action="store_true",
        help="Save the plot but do not open an interactive window",
    )
    args = parser.parse_args()

    if args.from_log:
        output = args.from_log.read_text()
    else:
        print("Running cargo bench --bench shprg_memory ...", file=sys.stderr)
        output = run_memory_bench()

    results = parse_results(output)
    print_breakdown(results)
    plot_overhead(results, args.output, show=not args.no_show)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
