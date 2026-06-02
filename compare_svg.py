import re, os, glob
from collections import Counter

mpl_dir = 'N238B W1-plots'
rs_dir = 'plots/N238B W1-plots'

print("=" * 80)
print("SVG 对比验证: Matplotlib vs Rsplotlib")
print("=" * 80)

# 收集所有SVG文件
mpl_files = sorted(glob.glob(os.path.join(mpl_dir, '*.svg')))
rs_files = sorted(glob.glob(os.path.join(rs_dir, '*.svg')))
print(f"\n文件数量: matplotlib={len(mpl_files)}, rsplotlib={len(rs_files)}")

# 详细对比第一个文件
if mpl_files and rs_files:
    mpl_file = mpl_files[0]
    rs_file = rs_files[0]
    name = os.path.basename(mpl_file)
    print(f"\n=== 对比文件: {name[:50]} ===")
    
    with open(mpl_file) as f:
        mpl = f.read()
    with open(rs_file) as f:
        rs = f.read()
    
    print(f"  大小: matplotlib={len(mpl)//1024}KB, rsplotlib={len(rs)//1024}KB")
    
    # viewBox
    vb_mpl = re.search(r'viewBox="([^"]+)"', mpl)
    vb_rs = re.search(r'viewBox="([^"]+)"', rs)
    print(f"  viewBox: matplotlib={vb_mpl.group(1) if vb_mpl else 'none'}, rsplotlib={vb_rs.group(1) if vb_rs else 'none'}")
    
    # 元素统计
    mpl_elements = Counter()
    rs_elements = Counter()
    for tag in ['path', 'line', 'rect', 'text', 'polyline', 'circle']:
        mpl_elements[tag] = len(re.findall(f'<{tag}', mpl))
        rs_elements[tag] = len(re.findall(f'<{tag}', rs))
    print(f"  matplotlib元素: {dict(mpl_elements)}")
    print(f"  rsplotlib元素: {dict(rs_elements)}")
    
    # 颜色分布 (stroke颜色)
    mpl_colors = Counter(re.findall(r'stroke:\s*#([0-9a-fA-F]{6})', mpl))
    rs_colors = Counter(re.findall(r'stroke="#([0-9a-fA-F]{6})"', rs))
    print(f"\n  matplotlib颜色(前8): {dict(list(mpl_colors.items())[:8])}")
    print(f"  rsplotlib颜色(前8): {dict(list(rs_colors.items())[:8])}")

# 所有文件的统计
print(f"\n{'='*80}")
print("所有文件统计")
print(f"{'='*80}")
print(f"{'文件名':<50} {'mpl(KB)':>8} {'rs(KB)':>8} {'mpl paths':>10} {'rs polylines':>12}")
print(f"{'-'*50} {'-'*8} {'-'*8} {'-'*10} {'-'*12}")

for mpl_file in mpl_files:
    name = os.path.basename(mpl_file)
    rs_file = os.path.join(rs_dir, name)
    if not os.path.exists(rs_file):
        continue
    
    with open(mpl_file) as f:
        mpl = f.read()
    with open(rs_file) as f:
        rs = f.read()
    
    mpl_paths = len(re.findall(r'<path', mpl))
    rs_polylines = len(re.findall(r'<polyline', rs))
    print(f"{name[:50]:<50} {len(mpl)//1024:>8} {len(rs)//1024:>8} {mpl_paths:>10} {rs_polylines:>12}")
