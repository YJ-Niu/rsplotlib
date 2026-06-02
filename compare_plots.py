#!/usr/bin/env python3
"""Convert SVG to PNG and compare matplotlib vs rsplotlib plots"""
import subprocess
import os
import sys

# Use cairosvg if available, otherwise rsvg-convert, otherwise just note the comparison
mpl_dir = '/Users/user/Desktop/rust_project/rsplotlib/N238B W1-plots'
rs_dir = '/Users/user/Desktop/rust_project/rsplotlib/plots/N238B W1-plots'
out_dir = '/tmp/plot_comparison'

os.makedirs(out_dir, exist_ok=True)

files = sorted(os.listdir(mpl_dir))

print("Comparing plots...")
for f in files[:5]:
    mpl_svg = os.path.join(mpl_dir, f)
    rs_svg = os.path.join(rs_dir, f)
    
    # Extract just the data lines from both SVGs to compare
    with open(mpl_svg) as fh:
        mpl_content = fh.read()
    with open(rs_svg) as fh:
        rs_content = fh.read()
    
    # Count data-related elements (non-grid lines)
    import re
    mpl_paths = len(re.findall(r'<path[^>]*stroke="[^"]*"[^>]*fill="none"', mpl_content))
    rs_polylines_with_stroke = len(re.findall(r'<polyline[^>]*stroke="[^"]*"[^>]*/>', rs_content))
    
    # Check title position
    mpl_title = re.search(r'<text[^>]*>[^<]*Txpower', mpl_content)
    rs_title = re.search(r'<text[^>]*>[^<]*Txpower', rs_content)
    
    # Check grid types
    mpl_grid_elements = len(re.findall(r'stroke="#[0-9A-Fa-f]+"[^/]*stroke-width="0\.[48]"', mpl_content))
    
    # Look at the data line positions in rsplotlib
    rs_data_lines = re.findall(r'<polyline fill="none" opacity="1" stroke="#(?:1431F5|72F64A|74F9FD|FEFB54|EA51F7)" stroke-width="[^"]*" points="([^"]*)"', rs_content)
    print(f"\n=== {f} ===")
    print(f"  matplotlib: {mpl_paths} data paths, grid elements: {mpl_grid_elements}")
    print(f"  rsplotlib:  {rs_polylines_with_stroke} total polylines, {len(rs_data_lines)} data line polylines")
    
    # Check if rsplotlib data lines are individual segments or continuous
    for i, pts_str in enumerate(rs_data_lines[:3]):
        pts = pts_str.split()
        n_pts = len(pts)
        print(f"  rsplotlib data line {i}: {n_pts} coordinate pairs")

print(f"\nConversion notes: to compare visually, use 'rsvg-convert -o output.png input.svg' or open in browser")