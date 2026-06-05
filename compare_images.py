"""Compare matplotlib and rsplotlib outputs."""
from PIL import Image, ImageChops
import os
import numpy as np
import sys

mpl_dir = 'N238B W1-plots'
rs_dir = 'plots/N238B W1-plots'

os.makedirs('diff_output', exist_ok=True)

mpl_files = sorted([f for f in os.listdir(mpl_dir) if f.endswith('.png')])
rs_files = sorted([f for f in os.listdir(rs_dir) if f.endswith('.png')])

# Pick a subset for the analysis: 0, 10, 11, 4
target_idx = [0, 4, 10, 11]

for idx in target_idx:
    m0_name = mpl_files[idx] if idx < len(mpl_files) else None
    r0_name = rs_files[idx] if idx < len(rs_files) else None
    if not m0_name or not r0_name:
        continue
    print(f"\n=== Image {idx}: {m0_name} ===")

    m0 = Image.open(os.path.join(mpl_dir, m0_name)).convert('RGB')
    r0 = Image.open(os.path.join(rs_dir, r0_name)).convert('RGB')

    # Side-by-side
    w, h = m0.size
    side = Image.new('RGB', (w*2 + 20, h), (200, 200, 200))
    side.paste(m0, (0, 0))
    side.paste(r0, (w+20, 0))
    side.save(f'diff_output/side_{idx}.png')

    # Diff
    diff = ImageChops.difference(m0, r0)
    diff.save(f'diff_output/diff_{idx}.png')

    arr_m = np.array(m0).astype(int)
    arr_r = np.array(r0).astype(int)
    d = np.abs(arr_m - arr_r).sum(axis=2)
    print(f"  Different pixels: {(d>0).sum()} / {d.size} ({(d>0).sum()/d.size*100:.1f}%)")
    print(f"  Mean diff: {d.mean():.2f}, Max diff: {d.max()}")

    # Per-row analysis
    row_diff = d.sum(axis=1) / w
    top_rows = sorted(range(h), key=lambda i: -row_diff[i])[:10]
    print(f"  Top 10 rows with most diff: {top_rows}")
    for r in top_rows[:5]:
        print(f"    y={r}: avg_diff={row_diff[r]:.1f}")

    # Per-column analysis
    col_diff = d.sum(axis=0) / h
    top_cols = sorted(range(w), key=lambda i: -col_diff[i])[:10]
    print(f"  Top 10 cols with most diff: {top_cols}")
    for c in top_cols[:5]:
        print(f"    x={c}: avg_diff={col_diff[c]:.1f}")
