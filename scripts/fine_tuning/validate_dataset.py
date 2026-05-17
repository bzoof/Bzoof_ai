#!/usr/bin/env python3
"""
Validate and analyze fine-tuning datasets for bwb_ai.

Usage:
    python3 validate_dataset.py <dataset.jsonl>
"""

import json
import sys
from collections import defaultdict
from pathlib import Path

def validate_record(record, index):
    """Validate a single record."""
    errors = []

    # Required fields
    if "instruction" not in record:
        errors.append(f"Row {index}: Missing 'instruction' field")
    if "output" not in record:
        errors.append(f"Row {index}: Missing 'output' field")

    # Field lengths
    instruction = record.get("instruction", "")
    output = record.get("output", "")

    if len(instruction) < 5:
        errors.append(f"Row {index}: Instruction too short (<5 chars)")
    if len(output) < 2:
        errors.append(f"Row {index}: Output too short (<2 chars)")

    if len(instruction) > 500:
        errors.append(f"Row {index}: Instruction too long (>500 chars)")
    if len(output) > 2000:
        errors.append(f"Row {index}: Output too long (>2000 chars)")

    return errors

def analyze_dataset(filepath):
    """Analyze dataset statistics."""
    records = []
    errors = []
    categories = defaultdict(int)
    total_instruction_tokens = 0
    total_output_tokens = 0

    print(f"Loading {filepath}...")

    try:
        with open(filepath, 'r') as f:
            for i, line in enumerate(f):
                try:
                    record = json.loads(line)
                    records.append(record)

                    # Validate
                    row_errors = validate_record(record, i + 1)
                    errors.extend(row_errors)

                    # Analyze
                    category = record.get("category", "unknown")
                    categories[category] += 1

                    # Simple token estimation (word count / 1.3)
                    instruction_tokens = len(record.get("instruction", "").split()) // 1.3
                    output_tokens = len(record.get("output", "").split()) // 1.3
                    total_instruction_tokens += instruction_tokens
                    total_output_tokens += output_tokens

                except json.JSONDecodeError as e:
                    errors.append(f"Row {i + 1}: Invalid JSON: {e}")

    except FileNotFoundError:
        print(f"ERROR: File not found: {filepath}")
        return

    # Print results
    print("\n" + "="*60)
    print(f"DATASET VALIDATION: {filepath}")
    print("="*60)

    print(f"\n✓ Total records: {len(records)}")
    print(f"✓ Categories: {dict(categories)}")
    print(f"✓ Avg instruction tokens: {total_instruction_tokens // max(1, len(records)):.1f}")
    print(f"✓ Avg output tokens: {total_output_tokens // max(1, len(records)):.1f}")
    print(f"✓ Total tokens: ~{(total_instruction_tokens + total_output_tokens):.0f}")

    if errors:
        print(f"\n✗ ERRORS FOUND ({len(errors)}):")
        for error in errors[:10]:  # Show first 10 errors
            print(f"  - {error}")
        if len(errors) > 10:
            print(f"  ... and {len(errors) - 10} more errors")
    else:
        print(f"\n✓ No errors found! Dataset is valid.")

    # Recommendations
    print("\n" + "="*60)
    print("RECOMMENDATIONS:")
    print("="*60)

    if len(records) < 50:
        print("⚠ Dataset is small (<50 examples). Collect more examples for better results.")
    elif len(records) < 200:
        print("⚠ Dataset is minimal (50-200 examples). Consider 200-500 for solid improvement.")
    elif len(records) < 500:
        print("✓ Dataset size is good (200-500 examples). Ready for 1-2 hour training.")
    else:
        print("✓ Dataset is large (500+ examples). Can train for 3-4 hours with better results.")

    # Check balance
    if categories:
        max_cat = max(categories.values())
        min_cat = min(categories.values())
        if max_cat > min_cat * 3:
            print(f"⚠ Imbalanced categories. Largest: {max_cat}, Smallest: {min_cat}. Consider balancing.")

    print("\nNext steps:")
    print("1. Fix any errors found above")
    print("2. Use Google Colab to run fine_tune_qwen.ipynb")
    print("3. Combine all datasets: shell + code + qa")
    print("4. Train LoRA adapter for 2-4 hours")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 validate_dataset.py <dataset.jsonl>")
        sys.exit(1)

    filepath = sys.argv[1]
    analyze_dataset(filepath)
