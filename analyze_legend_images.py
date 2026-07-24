"""Analyze PNG images related to legend rendering."""
import sys
import os
from PIL import Image

# Files to check
FILES = [
    "test/test_legend_minimal.png",
    "test/test_rf/test_data/test1.png",
]

def analyze_image(filepath):
    """Analyze a single PNG image."""
    abs_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), filepath)

    if not os.path.exists(abs_path):
        print(f"[MISSING] {filepath} -- 文件不存在")
        return

    print(f"\n{'='*60}")
    print(f"[EXISTS] {filepath}")
    print(f"  完整路径: {abs_path}")
    print(f"  文件大小: {os.path.getsize(abs_path)} bytes")

    try:
        img = Image.open(abs_path)
        print(f"  格式: {img.format}")
        print(f"  尺寸: {img.size} (宽={img.size[0]}, 高={img.size[1]})")
        print(f"  模式: {img.mode}")

        # Convert to RGBA for consistent analysis
        if img.mode != "RGBA":
            img = img.convert("RGBA")

        pixels = img.load()
        w, h = img.size

        # Analyze pixel data
        # Look at background color (corners)
        corners = [
            ("左上角", pixels[0, 0]),
            ("右上角", pixels[w-1, 0]),
            ("左下角", pixels[0, h-1]),
            ("右下角", pixels[w-1, h-1]),
        ]
        print(f"  四角像素 (RGBA):")
        for label, px in corners:
            print(f"    {label}: {px}")

        # Check if image is mostly blank (all same color)
        sample_pixels = []
        for y in range(0, h, max(1, h // 10)):
            for x in range(0, w, max(1, w // 10)):
                sample_pixels.append(pixels[x, y])

        unique_colors = set(sample_pixels)
        print(f"  采样点 ({len(sample_pixels)} 个): {len(unique_colors)} 种不同颜色")

        # Look for non-white/non-background pixels (content)
        # Assume background is the most common corner color
        bg_candidates = [pixels[0, 0], pixels[w-1, 0], pixels[0, h-1], pixels[w-1, h-1]]
        from collections import Counter
        bg_color = Counter(bg_candidates).most_common(1)[0][0]

        non_bg_count = 0
        for y in range(0, h, 2):
            for x in range(0, w, 2):
                if pixels[x, y] != bg_color:
                    non_bg_count += 1

        total_sampled = (w // 2) * (h // 2)
        pct = (non_bg_count / total_sampled) * 100 if total_sampled > 0 else 0
        print(f"  非背景像素占比: {pct:.1f}% (每2像素采样)")

        # Scan for legend area (bottom-right quadrant is typical for matplotlib legends)
        legend_region_w = w // 3
        legend_region_h = h // 3
        legend_colors = set()
        for y in range(h - legend_region_h, h):
            for x in range(w - legend_region_w, w):
                legend_colors.add(pixels[x, y])
        # Remove the background color
        legend_colors.discard(bg_color)
        print(f"  右下角图例区域 (宽={legend_region_w}, 高={legend_region_h}): {len(legend_colors)} 种非背景颜色")

        # Check top area for legend (matplotlib also puts legend at 'best' location)
        top_legend_colors = set()
        for y in range(0, legend_region_h):
            for x in range(0, w):
                top_legend_colors.add(pixels[x, y])
        top_legend_colors.discard(bg_color)
        print(f"  顶部图例区域: {len(top_legend_colors)} 种非背景颜色")

        # Count distinct non-background colors in the entire image
        all_colors = set()