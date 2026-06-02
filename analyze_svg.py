import os, re

def analyze_svg(filepath, label):
    with open(filepath, 'r') as f:
        content = f.read()
    
    lines = content.split('\n')
    print(f'=== {label}: {os.path.basename(filepath)} ===')
    print(f'  Lines: {len(lines)}, Size: {len(content)} bytes')
    
    rects = len(re.findall(r'<rect', content))
    lines_el = len(re.findall(r'<line', content))
    paths = len(re.findall(r'<path', content))
    polylines = len(re.findall(r'<polyline', content))
    polygons = len(re.findall(r'<polygon', content))
    texts = len(re.findall(r'<text', content))
    circles = len(re.findall(r'<circle', content))
    uses = len(re.findall(r'<use', content))
    g_tags = len(re.findall(r'<g ', content))
    print(f'  Elements: rect={rects} line={lines_el} path={paths} polyline={polylines}')
    print(f'            polygon={polygons} text={texts} circle={circles} use={uses} g={g_tags}')
    
    if paths > 0:
        path_styles = re.findall(r'style="[^"]*"', content)
        strokes = [s for s in path_styles if 'stroke' in s]
        fills = [s for s in path_styles if 'fill' in s and 'none' not in s]
        print(f'  Stroked paths: {len(strokes)}, Filled paths: {len(fills)}')
    
    if polylines > 0:
        poly_strokes = re.findall(r'stroke="([^"]*)"', content)
        from collections import Counter
        stroke_counts = Counter(poly_strokes)
        print(f'  Polyline stroke colors: {dict(stroke_counts.most_common(5))}')
    
    # Check for grid lines
    grid_lines = [l for l in lines if 'grid' in l.lower() or '787A78' in l or '808080' in l]
    print(f'  Grid-like elements: {len(grid_lines)}')
    
    # Check for data series
    data_colors = re.findall(r'stroke="#([0-9A-Fa-f]{6})"', content)
    from collections import Counter
    color_counts = Counter(data_colors)
    print(f'  Stroke colors (non-grid): {dict(color_counts.most_common(8))}')
    
    # Get first few lines
    print(f'  First 5 lines:')
    for l in lines[:5]:
        print(f'    {l[:120]}')
    print()

mpl_dir = '/Users/user/Desktop/rust_project/rsplotlib/N238B W1-plots'
rs_dir = '/Users/user/Desktop/rust_project/rsplotlib/plots/N238B W1-plots'

files = sorted(os.listdir(mpl_dir))
print(f'Comparing {len(files)} plot pairs')
print('='*60)

for f in files[:3]:
    mpl_path = os.path.join(mpl_dir, f)
    rs_path = os.path.join(rs_dir, f)
    if os.path.exists(mpl_path) and os.path.exists(rs_path):
        analyze_svg(mpl_path, 'MATPLOTLIB')
        analyze_svg(rs_path, 'RSPLOTLIB')
        print('---')