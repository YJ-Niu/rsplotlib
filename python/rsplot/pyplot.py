"""rsplot.pyplot - Matplotlib pyplot 兼容接口

此模块提供与 matplotlib.pyplot 兼容的 API，所有函数代理到 rsplot 核心模块。
使用方法: import rsplot.pyplot as plt
"""

from . import rsplot as _rsplot


def _get_axes():
    """获取当前 axes，如果没有则返回 None"""
    try:
        return _rsplot.gca()
    except Exception:
        return None


def _get_figure():
    """获取当前 figure，如果没有则返回 None"""
    try:
        return _rsplot.gcf()
    except Exception:
        return None


# ==================== 绘图函数 ====================

def plot(*args, **kwargs):
    """绘制折线图
    
    支持 matplotlib 兼容的关键字参数别名:
        lw: 线宽 (linewidth)
        c: 颜色 (color)
        ls: 线型 (linestyle)
    
    如果存在当前 axes，则复用它；否则创建新的 Figure 和 Axes。
    """
    _map_aliases(kwargs)
    x, y, kw = _parse_plot_args(args, kwargs)
    ax = _get_axes()
    if ax is not None:
        ax.plot(x, y, kw.get('label'), kw.get('color'), kw.get('linestyle'), kw.get('marker'), kw.get('linewidth'), kw.get('markersize'), kw.get('markeredgewidth'), kw.get('solid_capstyle'))
        return _get_figure()
    return _rsplot.plot(x, y, kw.get('label'), kw.get('color'), kw.get('linestyle'), kw.get('marker'), kw.get('linewidth'), kw.get('markersize'), kw.get('markeredgewidth'), kw.get('solid_capstyle'))


def scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0):
    """绘制散点图"""
    ax = _get_axes()
    if ax is not None:
        ax.scatter(x, y, s, c, marker, label, alpha)
        return _get_figure()
    return _rsplot.scatter(x, y, s, c, marker, label, alpha)


def bar(x, height, width=0.8, color=None, label=None):
    """绘制柱状图"""
    ax = _get_axes()
    if ax is not None:
        ax.bar(x, height, width, color, label)
        return _get_figure()
    return _rsplot.bar(x, height, width, color, label)


def barh(y, width, height=0.8, color=None, label=None):
    """绘制水平柱状图"""
    ax = _get_axes()
    if ax is not None:
        ax.barh(y, width, height, color, label)
        return _get_figure()
    return _rsplot.barh(y, width, height, color, label)


def hist(x, bins=10, density=False, label=None, alpha=0.7, color=None, **kwargs):
    """绘制直方图
    
    支持 matplotlib 兼容的参数:
        facecolor: 填充颜色 (优先级高于 color)
        align: 对齐方式 ('mid', 'left', 'right')
        histtype: 直方图类型 ('bar', 'step', 'stepfilled')
    
    如果存在当前 axes，则复用它；否则创建新的 Figure 和 Axes。
    """
    ax = _get_axes()
    facecolor = kwargs.pop('facecolor', None)
    align = kwargs.pop('align', None)
    histtype = kwargs.pop('histtype', None)
    _color = facecolor if facecolor is not None else color
    
    # 处理单数据集和多数据集
    if x and isinstance(x[0], (list, tuple)):
        x_list = [list(v) for v in x]
    else:
        x_list = [list(x)]
    
    # 处理颜色参数：字符串或列表
    if _color is not None:
        if isinstance(_color, str):
            color_list = [_color] * len(x_list)
        elif isinstance(_color, (list, tuple)):
            color_list = list(_color)
        else:
            color_list = None
    else:
        color_list = None
    
    if ax is not None:
        result = ax.hist(x_list, bins, density, label, alpha, color_list, None, align, histtype)
        return _get_figure()
    
    result = _rsplot.hist(x_list, bins, density, label, alpha, color_list)
    return result


def _unwrap_hist_result(ax, result):
    """将 hist 返回的 Vec<Vec<f64>> 包装成兼容 matplotlib 的形式
    
    当只有一个数据集时，将 n 从 [[...]] 解包为 [...]
    """
    n, bins, patches = result
    if len(n) == 1:
        return (n[0], bins, patches)
    return (n, bins, patches)


def pie(x, labels=None, colors=None, autopct=False, **kwargs):
    """绘制饼图"""
    autopct_str = None
    if autopct and isinstance(autopct, str):
        autopct_str = autopct
    elif autopct:
        autopct_str = "%1.1f%%"
    ax = _get_axes()
    if ax is not None:
        ax.pie(x, labels, colors, autopct_str)
        return _get_figure()
    return _rsplot.pie(x, labels, colors, autopct_str)


def boxplot(x, labels=None, vert=True, **kwargs):
    """绘制箱线图"""
    ax = _get_axes()
    if ax is not None:
        ax.boxplot(x, labels, vert)
        return _get_figure()
    return _rsplot.boxplot(x, labels, vert)


def fill_between(x, y1, y2=0.0, color=None, alpha=0.3, label=None, **kwargs):
    """填充区域"""
    ax = _get_axes()
    if ax is not None:
        ax.fill_between(x, y1, y2, color, alpha, label)
        return _get_figure()
    return _rsplot.fill_between(x, y1, y2, color, alpha, label)


def errorbar(x, y, yerr=None, xerr=None, fmt='o', color=None, label=None, capsize=3.0, **kwargs):
    """绘制误差棒图"""
    ax = _get_axes()
    if ax is not None:
        ax.errorbar(x, y, yerr, xerr, fmt, color, label, capsize)
        return _get_figure()
    return _rsplot.errorbar(x, y, yerr, xerr, fmt, color, label, capsize)


def stem(x, y, linefmt=None, markerfmt=None, label=None, **kwargs):
    """绘制茎叶图"""
    ax = _get_axes()
    if ax is not None:
        ax.stem(x, y, linefmt or '-', markerfmt or 'o', label)
        return _get_figure()
    return _rsplot.stem(x, y, linefmt or '-', markerfmt or 'o', label)


def step(x, y, where='pre', label=None, color=None, linestyle='-', linewidth=1.5, **kwargs):
    """绘制阶梯图"""
    ax = _get_axes()
    if ax is not None:
        ax.step(x, y, where, label, color, linestyle, linewidth)
        return _get_figure()
    return _rsplot.step(x, y, where, label, color, linestyle, linewidth)


def imshow(x, cmap='viridis', aspect='auto', **kwargs):
    """显示图像"""
    ax = _get_axes()
    if ax is not None:
        ax.imshow(x, cmap, aspect)
        return _get_figure()
    return _rsplot.imshow(x, cmap, aspect)


# ==================== 辅助元素 ====================

def text(x, y, s, fontdict=None, **kwargs):
    """添加文本"""
    fontsize = kwargs.get('fontsize', fontdict.get('fontsize', 12) if fontdict else 12)
    color = kwargs.get('color', fontdict.get('color', 'black') if fontdict else 'black')
    c = kwargs.get('c', None)
    family = kwargs.get('family', None)
    # Convert s to string to handle int/float types
    if not isinstance(s, str):
        s = str(s)
    return _rsplot.text(x, y, s, fontsize, color, c, family)


def axhline(y=0, **kwargs):
    """添加水平参考线"""
    return _rsplot.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axvline(x=0, **kwargs):
    """添加垂直参考线"""
    return _rsplot.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def hlines(y, xmin, xmax, **kwargs):
    """绘制水平线段"""
    return _rsplot.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def vlines(x, ymin, ymax, **kwargs):
    """绘制垂直线段"""
    return _rsplot.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


# ==================== 配置函数 ====================

def xlabel(text, **kwargs):
    """设置 x 轴标签"""
    return _rsplot.xlabel(text)


def ylabel(text, **kwargs):
    """设置 y 轴标签"""
    return _rsplot.ylabel(text)


def title(label, fontdict=None, **kwargs):
    """设置图表标题"""
    return _rsplot.title(label)


def grid(visible=True, **kwargs):
    """显示/隐藏网格"""
    c = kwargs.get('c')
    ls = kwargs.get('linestyle') or kwargs.get('ls')
    lw = kwargs.get('linewidth') or kwargs.get('lw')
    axis = kwargs.get('axis')
    return _rsplot.grid(visible, c, ls, lw, axis)


def legend(loc='best', **kwargs):
    """显示图例"""
    return _rsplot.legend(loc)


def xlim(left=None, right=None, **kwargs):
    """设置/获取 x 轴范围"""
    return _rsplot.xlim(left, right)


def ylim(bottom=None, top=None, **kwargs):
    """设置/获取 y 轴范围"""
    return _rsplot.ylim(bottom, top)


def xticks(ticks=None, labels=None, **kwargs):
    """设置 x 轴刻度"""
    return _rsplot.xticks(ticks, labels)


def yticks(ticks=None, labels=None, **kwargs):
    """设置 y 轴刻度"""
    return _rsplot.yticks(ticks, labels)


def xscale(scale, **kwargs):
    """设置 x 轴缩放"""
    return _rsplot.xscale(scale)


def yscale(scale, **kwargs):
    """设置 y 轴缩放"""
    return _rsplot.yscale(scale)


def margins(x_margin=None, y_margin=None, **kwargs):
    """设置自动缩放的边距"""
    return _rsplot.margins(x_margin, y_margin)


def box(on=None):
    """设置坐标轴边框"""
    return _rsplot.box_(on)


def minorticks_on():
    """显示次要刻度"""
    return _rsplot.minorticks_on()


def minorticks_off():
    """隐藏次要刻度"""
    return _rsplot.minorticks_off()


# ==================== 子图与布局 ====================

def subplots(nrows=1, ncols=1, **kwargs):
    """创建子图网格"""
    return _rsplot.subplots(nrows, ncols)


def subplot(nrows, ncols, index, **kwargs):
    """创建单个子图"""
    return _rsplot.subplot(nrows, ncols, index)


def tight_layout(**kwargs):
    """自动调整子图布局"""
    return _rsplot.tight_layout()


def subplots_adjust(left=None, right=None, bottom=None, top=None, wspace=None, hspace=None):
    """调整子图布局参数"""
    fig = _get_current_figure()
    if fig is not None:
        fig.subplots_adjust(left, right, bottom, top, wspace, hspace)


def set_size(width, height):
    """设置图形尺寸"""
    return _rsplot.set_size(width, height)


def twinx():
    """创建共享 x 轴的双 y 轴"""
    return _rsplot.twinx()


def twiny():
    """创建共享 y 轴的双 x 轴"""
    return _rsplot.twiny()


# ==================== 图形控制 ====================

def figure(num=None, figsize=None, dpi=None, **kwargs):
    """创建新图形
    
    Args:
        num: 图形编号 (未使用, 兼容 matplotlib)
        figsize: (width, height) 元组，单位为英寸
        dpi: 分辨率
    """
    fig = _rsplot.figure()
    d = dpi if dpi is not None else 100
    fig.set_dpi(d)
    if figsize is not None:
        w_inch, h_inch = figsize
        fig.set_size(round(w_inch * d), round(h_inch * d))
    else:
        w, h = rcParams.get('figure.figsize', [6.4, 4.8])
        fig.set_size(round(w * d), round(h * d))
    return fig


def savefig(fname, **kwargs):
    """保存图形"""
    _rsplot.savefig(fname)
    if isinstance(fname, str) and fname.endswith('.svg'):
        _post_process_svg(fname)


def _post_process_svg(filepath):
    """Post-process SVG to convert polyline/line/circle elements to path elements
    to match matplotlib's SVG output format.
    """
    import re

    with open(filepath, 'r') as f:
        content = f.read()

    modified = False

    def replace_polyline(m):
        nonlocal modified
        modified = True
        attrs_before = m.group(1)
        points_str = m.group(2)
        attrs_after = m.group(3)
        pairs = re.findall(r'([\d.-]+),([\d.-]+)', points_str)
        if not pairs:
            return m.group(0)
        d_parts = []
        for i, (x, y) in enumerate(pairs):
            cmd = 'M' if i == 0 else 'L'
            d_parts.append('%s %s %s' % (cmd, x, y))
        attrs = attrs_before + attrs_after
        return '<path d="%s"%s/>' % (' '.join(d_parts), attrs)

    content = re.sub(
        r'<polyline([^>]*)points="([^"]+)"([^>]*)/>',
        replace_polyline,
        content
    )

    def replace_line(m):
        nonlocal modified
        modified = True
        attrs_before_x1 = m.group(1)
        x1 = m.group(2)
        attrs_between_x1_y1 = m.group(3)
        y1 = m.group(4)
        attrs_between_y1_x2 = m.group(5)
        x2 = m.group(6)
        attrs_between_x2_y2 = m.group(7)
        y2 = m.group(8)
        attrs_after_y2 = m.group(9)
        attrs = attrs_before_x1 + attrs_between_x1_y1 + attrs_between_y1_x2 + attrs_between_x2_y2 + attrs_after_y2
        return '<path d="M %s %s L %s %s"%s/>' % (x1, y1, x2, y2, attrs)

    content = re.sub(
        r'<line([^>]*)x1="([^"]+)"([^>]*)y1="([^"]+)"([^>]*)x2="([^"]+)"([^>]*)y2="([^"]+)"([^>]*)/>',
        replace_line,
        content
    )

    def replace_circle(m):
        nonlocal modified
        modified = True
        attrs_before_cx = m.group(1)
        cx = float(m.group(2))
        attrs_between_cx_cy = m.group(3)
        cy = float(m.group(4))
        attrs_between_cy_r = m.group(5)
        r = float(m.group(6))
        attrs_after_r = m.group(7)
        attrs = attrs_before_cx + attrs_between_cx_cy + attrs_between_cy_r + attrs_after_r
        d = 'M %s %s A %s %s 0 1 0 %s %s A %s %s 0 1 0 %s %s' % (cx, cy - r, r, r, cx, cy + r, r, r, cx, cy - r)
        return '<path d="%s"%s/>' % (d, attrs)

    content = re.sub(
        r'<circle([^>]*)cx="([^"]+)"([^>]*)cy="([^"]+)"([^>]*)r="([^"]+)"([^>]*)/>',
        replace_circle,
        content
    )

    if modified:
        content = _consolidate_paths(content)
        content = _convert_to_style_format(content)
        with open(filepath, 'w') as f:
            f.write(content)


def _consolidate_paths(content):
    """Consolidate adjacent path elements with identical style attributes.
    
    Group consecutive path elements that have the same attributes (style, fill,
    stroke, etc.) and combine their 'd' attributes into a single path element
    with multiple subpaths. This reduces file size and matches matplotlib's
    SVG output style.
    """
    import re
    path_re = r'<path\s+d="([^"]*)"([^>]*)/>'
    parts = []
    last_end = 0
    i = 0
    while True:
        path_re_compiled = re.compile(path_re)
        m = path_re_compiled.search(content, i)
        if not m:
            break
        d = m.group(1)
        attrs = m.group(2)
        group_ds = [d]
        j = m.end()
        while True:
            next_m = path_re_compiled.search(content, j)
            if next_m and next_m.group(2) == attrs:
                group_ds.append(next_m.group(1))
                j = next_m.end()
            else:
                break
        if len(group_ds) > 1:
            combined = ' '.join(group_ds)
            parts.append(content[i:m.start()])
            parts.append('<path d="%s"%s/>' % (combined, attrs))
            i = j
        else:
            i = m.end()
    if i < len(content):
        parts.append(content[i:])
    result = ''.join(parts) if parts else content
    return result


def _convert_to_style_format(content):
    """Convert fill/stroke/opacity/stroke-width attributes to style="..." format
    to match matplotlib's SVG output style.
    """
    import re
    
    style_attrs = {'fill', 'stroke', 'stroke-width', 'opacity', 'stroke-linecap'}
    
    def convert_attrs(m):
        prefix = m.group(1)
        attrs_str = m.group(2)
        closer = m.group(3)
        
        style_parts = []
        other_attrs = []
        
        for attr_match in re.finditer(r'(\S+)\s*=\s*"([^"]*)"', attrs_str):
            name = attr_match.group(1)
            value = attr_match.group(2)
            if name in style_attrs:
                css_name = name
                style_parts.append('%s: %s' % (css_name, value))
            else:
                other_attrs.append('%s="%s"' % (name, value))
        
        result = '<' + prefix
        if other_attrs:
            result += ' ' + ' '.join(other_attrs)
        if style_parts:
            result += ' style="' + '; '.join(style_parts) + '"'
        result += closer
        return result
    
    content = re.sub(
        r'<(path|rect|circle|ellipse|line|polyline|polygon|text)([^>]*)(/?>)',
        convert_attrs,
        content
    )
    
    return content


def show(**kwargs):
    """显示图形"""
    return _rsplot.show()


def gca(**kwargs):
    """获取当前 Axes"""
    return _rsplot.gca()


def gcf(**kwargs):
    """获取当前 Figure"""
    return _rsplot.gcf()


def cla():
    """清空当前 Axes"""
    return _rsplot.cla()


def clf():
    """清空当前 Figure"""
    return _rsplot.clf()


def close(fig=None):
    """关闭图形
    
    Args:
        fig: 图形或 'all' (兼容 matplotlib)
    """
    return _rsplot.close()


def axis(arg=None, **kwargs):
    """坐标轴控制
    
    支持:
        axis('off'): 隐藏坐标轴
        axis('equal'): 等比例
        axis('tight'): 紧凑
    """
    if arg == 'off':
        _set_axis_off()
    elif arg == 'equal' or arg == 'scaled':
        gca().set_aspect('equal')
    return None


def colorbar(mappable=None, **kwargs):
    """添加颜色条 (占位)"""
    pass


# ==================== 当前图形/坐标轴辅助 ====================

def _get_current_figure():
    try:
        return _rsplot.gcf()
    except:
        return None


def _set_axis_off():
    try:
        ax = _rsplot.gca()
        ax._axis_off()
    except:
        pass


def _map_aliases(kwargs):
    alias_map = {
        'lw': 'linewidth',
        'c': 'color',
        'ls': 'linestyle',
        'marker': 'marker',
        'label': 'label',
    }
    for alias, target in alias_map.items():
        if alias in kwargs and target not in kwargs:
            kwargs[target] = kwargs.pop(alias)
        elif alias in kwargs:
            kwargs.pop(alias)


def _parse_plot_args(args, kwargs):
    if len(args) == 2:
        return args[0], args[1], kwargs
    elif len(args) == 1:
        return range(len(args[0])), args[0], kwargs
    return [], [], kwargs


# ==================== rcParams 支持 ====================

class RcParams(dict):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.update({
            'font.sans-serif': ['Helvetica', 'Arial', 'sans-serif'],
            'axes.unicode_minus': True,
            'font.size': 10,
            'figure.figsize': [6.4, 4.8],
            'figure.dpi': 100.0,
        })

    def __getitem__(self, key):
        try:
            return super().__getitem__(key)
        except KeyError:
            return None

    def __setitem__(self, key, value):
        super().__setitem__(key, value)


rcParams = RcParams()
rcParamsOrig = RcParams()


def get_cmap(name=None, lut=None):
    """获取颜色映射 (占位)"""
    return name
