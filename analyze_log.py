#!/usr/bin/env python3
"""Analyze tempo debug logs and emit summary statistics."""

import argparse
import json
import re
from datetime import datetime
from statistics import mean, median, stdev
from pathlib import Path


def parse_time_to_ms(time_str):
    """
    Convert time strings to milliseconds.
    Handles formats like: 1.186125ms, 11.583µs, 2.666375ms, 180.042292ms, etc.
    """
    time_str = time_str.strip()

    # Match patterns like "123.456ms" or "789.012µs" or "1.234s"
    match = re.match(r'([\d.]+)(ms|µs|s)', time_str)
    if not match:
        return None

    value, unit = match.groups()
    value = float(value)

    # Convert to milliseconds
    if unit == 'ms':
        return value
    elif unit == 'µs':
        return value / 1000.0
    elif unit == 's':
        return value * 1000.0

    return None


def strip_ansi_codes(text):
    """Remove ANSI escape codes from text."""
    ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
    return ansi_escape.sub('', text)


def parse_timestamp(line):
    """Extract timestamp from log line."""
    # Strip ANSI codes first
    clean_line = strip_ansi_codes(line)
    # Match ISO timestamp at start of line: 2025-09-29T22:22:37.272569Z
    match = re.match(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z)', clean_line)
    if match:
        return datetime.fromisoformat(match.group(1).replace('Z', '+00:00'))
    return None


def find_block_range(log_file, min_gas=1000):
    """
    Find the range of non-empty blocks in the log.
    Returns (first_block_num, last_block_num) or (None, None) if no non-empty blocks found.
    """
    non_empty_blocks = []

    with open(log_file, 'r', encoding='utf-8') as f:
        for line in f:
            clean_line = strip_ansi_codes(line)

            if 'Block added to canonical chain' in clean_line:
                # Extract block number and gas used
                num_match = re.search(r'number\s*=\s*(\d+)', clean_line)
                gas_match = re.search(r'gas_used\s*=\s*([\d.]+)([KMG]?)gas', clean_line)

                if num_match and gas_match:
                    block_num = int(num_match.group(1))
                    gas_val = float(gas_match.group(1))
                    gas_unit = gas_match.group(2)

                    # Convert to actual gas units
                    if gas_unit == 'K':
                        gas_used = gas_val * 1000
                    elif gas_unit == 'M':
                        gas_used = gas_val * 1000000
                    elif gas_unit == 'G':
                        gas_used = gas_val * 1000000000
                    else:
                        gas_used = gas_val

                    if gas_used > min_gas:
                        non_empty_blocks.append(block_num)

    if len(non_empty_blocks) >= 3:
        # Exclude first and last non-empty blocks (ramp-up/ramp-down)
        # Start from block after first, end at block before last
        return (non_empty_blocks[0] + 1, non_empty_blocks[-1] - 1)
    elif non_empty_blocks:
        # If we have less than 3 blocks, just use them all
        return (non_empty_blocks[0], non_empty_blocks[-1])
    return (None, None)


def parse_log_file(log_file, block_range=None):
    """
    Parse the debug.log file and extract timing information.

    Args:
        log_file: Path to the log file
        block_range: Optional tuple (first_block, last_block) to filter blocks
    """

    build_times = []
    explicit_state_root_times = []
    block_added_times = []
    payload_to_received_times = []  # Time from "Built payload" to "Received block"

    # Track "Built payload" times by parent block number
    # We use parent_number because the next block will be parent_number + 1
    built_payload_times = {}  # block_number -> timestamp

    with open(log_file, 'r', encoding='utf-8') as f:
        for line in f:
            timestamp = parse_timestamp(line)
            # Strip ANSI codes for easier pattern matching
            clean_line = strip_ansi_codes(line)

            # Parse "Built payload" lines for build time and track timestamp
            if 'Built payload' in clean_line:
                # Extract parent_number from build_payload{...parent_number=9672...}
                parent_match = re.search(r'parent_number\s*=\s*(\d+)', clean_line)
                if parent_match and timestamp:
                    parent_number = int(parent_match.group(1))
                    block_number = parent_number + 1
                    # Skip block 1
                    if block_number != 1:
                        built_payload_times[block_number] = timestamp

                # Extract elapsed time
                match = re.search(r'elapsed\s*=\s*([\d.]+(?:ms|µs|s))', clean_line)
                if match and parent_match:
                    block_number = int(parent_match.group(1)) + 1
                    # Check if block is in range
                    if block_range is None or (block_range[0] <= block_number <= block_range[1]):
                        time_ms = parse_time_to_ms(match.group(1))
                        if time_ms is not None:
                            build_times.append(time_ms)

            # Parse "Received block from consensus engine" and calculate time from "Built payload"
            elif 'Received block from consensus engine' in clean_line:
                number_match = re.search(r'number\s*=\s*(\d+)', clean_line)
                if number_match and timestamp:
                    block_number = int(number_match.group(1))
                    # Skip block 1 and check if in range
                    if (block_number != 1 and block_number in built_payload_times and
                        (block_range is None or (block_range[0] <= block_number <= block_range[1]))):
                        start_time = built_payload_times[block_number]
                        elapsed_ms = (timestamp - start_time).total_seconds() * 1000
                        payload_to_received_times.append(elapsed_ms)
                        # Clean up to save memory
                        del built_payload_times[block_number]

            # Parse "State root task finished" lines (if present in some logs)
            elif 'State root task finished' in clean_line:
                match = re.search(r'elapsed\s*=\s*([\d.]+(?:ms|µs|s))', clean_line)
                if match:
                    # Note: State root lines don't have block numbers, so we can't filter them perfectly
                    # They will be included based on chronological proximity to filtered blocks
                    time_ms = parse_time_to_ms(match.group(1))
                    if time_ms is not None:
                        explicit_state_root_times.append(time_ms)

            # Parse "Block added to canonical chain" lines
            elif 'Block added to canonical chain' in clean_line:
                number_match = re.search(r'number\s*=\s*(\d+)', clean_line)
                match = re.search(r'elapsed\s*=\s*([\d.]+(?:ms|µs|s))', clean_line)
                if match and number_match:
                    block_number = int(number_match.group(1))
                    # Check if block is in range
                    if block_range is None or (block_range[0] <= block_number <= block_range[1]):
                        time_ms = parse_time_to_ms(match.group(1))
                        if time_ms is not None:
                            block_added_times.append(time_ms)

    return build_times, explicit_state_root_times, payload_to_received_times, block_added_times


def compute_statistics(times):
    """Return statistics for a given set of timing measurements."""
    if not times:
        return None

    stats = {
        "count": len(times),
        "mean": mean(times),
        "median": median(times),
        "min": min(times),
        "max": max(times),
        "std_dev": stdev(times) if len(times) > 1 else 0.0,
    }
    return stats


def format_stats(name, stats):
    if not stats:
        print(f"\n{name}: No data found")
        return

    print(f"\n{name}:")
    print(f"  Count:   {stats['count']}")
    print(f"  Mean:    {stats['mean']:.3f} ms")
    print(f"  Median:  {stats['median']:.3f} ms")
    print(f"  Min:     {stats['min']:.3f} ms")
    print(f"  Max:     {stats['max']:.3f} ms")
    print(f"  Std Dev: {stats['std_dev']:.3f} ms")


def build_summary(log_file, block_range, build_times, explicit_state_root_times, payload_to_received_times, block_added_times, label=None):
    return {
        "label": label,
        "log_file": str(log_file),
        "block_range": block_range,
        "metrics": {
            "Build Payload Time": compute_statistics(build_times),
            "State Root Computation": compute_statistics(payload_to_received_times),
            "Explicit State Root Task": compute_statistics(explicit_state_root_times),
            "Block Added to Canonical Chain": compute_statistics(block_added_times),
        },
    }


def parse_args():
    parser = argparse.ArgumentParser(description="Analyze tempo debug logs for benchmark metrics.")
    parser.add_argument("--log", type=Path, default=Path(__file__).parent / "debug.log", help="Path to the log file to analyze.")
    parser.add_argument("--json", type=Path, help="Optional path to write summary statistics as JSON.")
    parser.add_argument("--label", help="Optional label to include in the JSON summary.")
    parser.add_argument("--quiet", action="store_true", help="Suppress detailed textual output.")
    return parser.parse_args()


def main():
    args = parse_args()

    log_file = args.log
    if not log_file.exists():
        print(f"Error: {log_file} not found")
        return

    if not args.quiet:
        print(f"Analyzing {log_file}...")

    first_block, last_block = find_block_range(log_file)
    block_range = None

    if first_block is not None:
        block_range = (first_block, last_block)
        if not args.quiet:
            print(f"Analyzing blocks: {first_block} to {last_block} ({last_block - first_block + 1} blocks)")
            print("(Excluding first/last non-empty blocks for ramp-up/down)\n")
    elif not args.quiet:
        print("No non-empty blocks found, analyzing all blocks...\n")

    build_times, explicit_state_root_times, payload_to_received_times, block_added_times = parse_log_file(
        log_file, block_range
    )

    summary = build_summary(
        log_file=str(log_file),
        block_range=block_range,
        build_times=build_times,
        explicit_state_root_times=explicit_state_root_times,
        payload_to_received_times=payload_to_received_times,
        block_added_times=block_added_times,
        label=args.label,
    )

    if not args.quiet:
        print("\n" + "=" * 60)
        print("LOG ANALYSIS RESULTS")
        print("=" * 60)
        format_stats("Build Payload Time", summary["metrics"]["Build Payload Time"])
        format_stats("State Root Computation", summary["metrics"]["State Root Computation"])
        format_stats("Explicit State Root Task", summary["metrics"]["Explicit State Root Task"])
        format_stats("Block Added to Canonical Chain", summary["metrics"]["Block Added to Canonical Chain"])
        print("\n" + "=" * 60)

    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(json.dumps(summary, indent=2))

    return summary


if __name__ == "__main__":
    main()
