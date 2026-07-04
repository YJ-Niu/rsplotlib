"""rsplotlib.pyplot - Matplotlib pyplot 兼容接口

此模块提供与 matplotlib.pyplot 兼容的 API，所有函数代理到 rsplotlib 核心模块。
使用方法: import rsplotlib.pyplot as plt
"""

from . import rsplotlib as _rsplotlib
from .figure._defaults import DEFAULT_DPI, DEFAULT_FIGSIZE
# ============ 样式接口 ============
from .utils import style as _style_module

# 延迟获取 mpl.rcParams，避免 pyplot <-> pylab 循环导入


def _get_rcparams():
    """从 pylab.mpl 获取 rcParams，统一配置入口"""
    from .pylab import mpl
    return mpl.rcParams


# ==================== 内部辅助函数 ====================

def _to_list(obj):
    """将 numpy 数组或其他可迭代对象转换为 Python list

    支持 numpy ndarray、Python list、tuple 及其他可迭代对象。
    标量值直接返回。
    """
    if obj is None:
        return None
    # numpy ndarray
    if hasattr(obj, 'tolist'):
        return obj.tolist()
    # Python list/tuple 或其他可迭代对象
    if isinstance(obj, (list, tuple)):
        return list(obj)
    # 标量
    return obj


def _to_list_recursive(obj):
    """递归转换嵌套的 numpy 数组为 Python list"""
    if obj is None:
        return None
    if hasattr(obj, 'tolist'):
        return obj.tolist()
    if isinstance(obj, (list, tuple)):
        return [_to_list_recursive(item) for item in obj]
    return obj


def _is_scatter_sequence(obj):
    """判断是否为序列（含 numpy 数组），但排除字符串标量。"""
    if obj is None or isinstance(obj, str):
        return False
    return hasattr(obj, 'tolist') or isinstance(obj, (list, tuple))


def _rgba_to_hex(row):
    """将 (r, g, b[, a]) (0-1 浮点) 转为 '#rrggbb' 十六进制颜色。"""
    row = list(row)

    def _ch(v):
        return max(0, min(255, int(round(float(v) * 255))))

    return '#{:02x}{:02x}{:02x}'.format(_ch(row[0]), _ch(row[1]), _ch(row[2]))


def _resolve_scatter_colors(c_vals, cmap, vmin, vmax):
    """把 c 序列解析为逐点颜色字符串列表 + colorbar 所需的 mappable 元数据。

    返回 (colors, mappable):
    - 颜色字符串序列 / RGB(A) 二维行数组: mappable 为 None
    - 数值序列: 经 colormap 映射为 hex，mappable = (cmap名, vmin, vmax)
    """
    if len(c_vals) == 0:
        return None, None
    first = c_vals[0]
    if _is_scatter_sequence(first):  # RGB(A) 二维行数组
        return [_rgba_to_hex(row) for row in c_vals], None
    if isinstance(first, str):       # 颜色字符串序列
        return [str(v) for v in c_vals], None
    vals = [float(v) for v in c_vals]  # 数值 -> colormap
    name = cmap if isinstance(cmap, str) else 'viridis'
    lo = min(vals) if vmin is None else float(vmin)
    hi = max(vals) if vmax is None else float(vmax)
    colors = list(_rsplotlib.colormap_hex(vals, name, lo, hi))
    return colors, (name, lo, hi)


def _normalize_scatter(x, y, s, c, marker, label, alpha, kwargs):
    """将 matplotlib 风格的 scatter 参数规整为对 Rust 层的调用参数。

    返回 (use_multi, args, mappable):
    - use_multi=False: args = (x, y, s:float, c:str|None, marker, label, alpha)
    - use_multi=True:  args = (x, y, s:list|None, c:list|None, marker, label, alpha)
    - mappable: None 或 (cmap名, vmin, vmax)，当 c 为数值数组经 colormap 映射时给出，
      供随后的 plt.colorbar() 绘制颜色条。
    """
    x = _to_list(x)
    y = _to_list(y)
    n = len(x) if hasattr(x, '__len__') else 0
    marker = marker or 'o'
    if c is None:
        c = kwargs.pop('color', None)
    cmap = kwargs.pop('cmap', None)
    vmin = kwargs.pop('vmin', None)
    vmax = kwargs.pop('vmax', None)
    # linewidths / edgecolors / norm / plotnonfinite / data 等参数当前接受但不生效

    s_is_seq = _is_scatter_sequence(s)
    c_is_seq = _is_scatter_sequence(c)

    c_list = None
    c_single = None
    mappable = None
    if c_is_seq:
        c_vals = _to_list(c)
        # 单个 RGB(A)（长度 3/4 的纯数值序列，且与点数不同）视为统一单色
        is_flat_numeric = all(
            not isinstance(v, str) and not _is_scatter_sequence(v) for v in c_vals
        )
        if is_flat_numeric and len(c_vals) in (3, 4) and len(c_vals) != n:
            c_single = _rgba_to_hex(c_vals)
        else:
            c_list, mappable = _resolve_scatter_colors(c_vals, cmap, vmin, vmax)
    elif isinstance(c, str):
        c_single = c

    use_multi = s_is_seq or (c_list is not None)
    if not use_multi:
        s_val = 100.0 if s is None else float(s)
        return False, (x, y, s_val, c_single, marker, label, alpha), mappable

    if s_is_seq:
        s_arg = [float(v) for v in _to_list(s)]
    elif s is None:
        s_arg = None
    else:
        s_arg = [float(s)] * n

    if c_list is not None:
        c_arg = c_list
    elif c_single is not None:
        c_arg = [c_single] * n
    else:
        c_arg = None
    return True, (x, y, s_arg, c_arg, marker, label, alpha), mappable


def _get_axes():
    """获取当前 axes，如果没有则返回 None"""
    try:
        return _rsplotlib.gca()
    except Exception:
        return None


def _get_figure():
    """获取当前 figure，如果没有则返回 None"""
    try:
        return _rsplotlib.gcf()
    except Exception:
        return None


def _route_to_ax(ax_method_name, module_method, *args, **kwargs):
    """将调用路由到当前 axes（如果存在）或模块级函数

    Args:
        ax_method_name: axes 的方法名（字符串）
        module_method: 模块级函数（可调用对象）
        *args: 传递给方法的参数
        **kwargs: 关键字参数（必须同时转发到 axes 端，否则会丢参）
    """
    ax = _get_axes()
    if ax is not None and hasattr(ax, ax_method_name):
        method = getattr(ax, ax_method_name)
        method(*args, **kwargs)
        return _get_figure()
    return module_method(*args, **kwargs)


def _map_aliases(kwargs):
    """规范化 matplotlib 别名到标准名"""
    alias_map = {
        'lw': 'linewidth',
        'c': 'color',
        'ls': 'linestyle',
        'ms': 'markersize',
        'mfc': 'markerfacecolor',
        'mec': 'markeredgecolor',
        'mew': 'markeredgewidth',
    }
    for alias, target in alias_map.items():
        if alias in kwargs and target not in kwargs:
            kwargs[target] = kwargs.pop(alias)
        elif alias in kwargs:
            kwargs.pop(alias)
    # 规范化 linestyle 词形 ('solid'/'dotted'/'dashed'/'dashdot') 与空值到简写，
    # 与 matplotlib 一致：既可写 linestyle='dotted' 也可写 linestyle=':'。
    ls = kwargs.get('linestyle')
    if isinstance(ls, str):
        key = ls.strip().lower()
        if key == '' or key == 'none':
            kwargs['linestyle'] = ' '  # 空串 / ' ' / 'None' 均表示不画线
        elif key in _LINESTYLE_ALIASES:
            kwargs['linestyle'] = _LINESTYLE_ALIASES[key]
        # 已是简写 ('-' / '--' / ':' / '-.') 时保持不变


# linestyle 词形 -> 简写。空串 / 'None' 在 _map_aliases 中单独处理为 ' '(不画线)。
_LINESTYLE_ALIASES = {
    'solid': '-',
    'dotted': ':',
    'dashed': '--',
    'dashdot': '-.',
}


def _parse_fmt(fmt):
    """解析 matplotlib 风格的格式字符串 '[marker][line][color]'。

    fmt 由三部分任意组合而成 (均可省略):
        marker: 数据点标记, 如 'o' 圆, '^' 三角, 's' 方块, '*' 星 ...
        line:   线型, '-' 实线, '--' 虚线, '-.' 点划线, ':' 点线
        color:  颜色单字母代码 b/g/r/c/m/y/k/w

    例如 'o:r' = 圆形标记 + 点线 + 红色。

    注意 black 与 blue 首字母冲突: 按 matplotlib 约定,
    单字母代码里 'b' 表示 blue, 'k' 才表示 black, 因此不会产生歧义。

    返回 dict, 可能包含 'marker' / 'linestyle' / 'color' 键。
    """
    if not fmt:
        return {}
    line_styles_multi = ('--', '-.')
    line_styles_single = ('-', ':')
    markers = set(".,ov^<>12348spP*hH+xXDd|_")
    color_map = {
        'b': 'blue', 'g': 'green', 'r': 'red', 'c': 'cyan',
        'm': 'magenta', 'y': 'yellow', 'k': 'black', 'w': 'white',
    }

    marker = linestyle = color = None
    i, n = 0, len(fmt)
    while i < n:
        two = fmt[i:i + 2]
        ch = fmt[i]
        if two in line_styles_multi:
            if linestyle is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的线型")
            linestyle, i = two, i + 2
        elif ch in line_styles_single:
            if linestyle is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的线型")
            linestyle, i = ch, i + 1
        elif ch in markers:
            if marker is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的标记")
            marker, i = ch, i + 1
        elif ch in color_map:
            if color is not None:
                raise ValueError(f"格式字符串 {fmt!r} 中出现了重复的颜色")
            color, i = color_map[ch], i + 1
        else:
            raise ValueError(f"无法识别的格式字符串 {fmt!r} (非法字符 {ch!r})")

    result = {}
    if marker is not None:
        result['marker'] = marker
    if linestyle is not None:
        result['linestyle'] = linestyle
    if color is not None:
        result['color'] = color
    return result


def _parse_plot_args(args, kwargs):
    """解析 plot() 的位置参数为 [(x, y, fmt_or_none), ...] 对列表 + kwargs

    支持的调用形式 (与 matplotlib 一致):
        plot(y)                    plot(x, y)
        plot(y, fmt)               plot(x, y, fmt)
        plot(x1, y1, fmt1, x2, y2, fmt2, ...)   # 多条线, 每组 fmt 可选
    """
    args = list(args)
    pairs = []
    while args:
        # 取出本组数据参数 (1~2 个); 若第 2 个其实是 fmt 字符串, 则本组只有 1 个
        group = args[:2]
        if len(group) == 2 and isinstance(group[1], str):
            group = group[:1]
        args = args[len(group):]
        # 消费紧跟其后的 fmt 字符串 (若有)
        fmt = None
        if args and isinstance(args[0], str):
            fmt, args = args[0], args[1:]

        if len(group) == 1:
            y = group[0]
            try:
                x = list(range(len(y)))
            except Exception:
                x = list(y) if hasattr(y, '__iter__') else []
            pairs.append((x, y, fmt))
        else:
            pairs.append((group[0], group[1], fmt))
    return pairs, kwargs


# ==================== 绘图函数 ====================

def plot(*args, **kwargs):
    """绘制折线图。

    用法:
        plt.plot(x, y)              # 以 x 为横坐标, y 为纵坐标
        plt.plot(y)                 # 仅提供 y, 自动 x = [0, 1, ...]
        plt.plot(x, y, lw=2.0)     # 自定义线宽
        plt.plot(x, y, x, z)       # 绘制多条线
        plt.plot(y, 'o:r')         # 格式字符串: 圆标记 + 点线 + 红色

    格式字符串 fmt = '[marker][line][color]', 各部分均可省略, 例如:
        'o'   仅圆形标记        '--'  仅虚线
        'o:r' 圆标记+点线+红色  '^-g' 三角标记+实线+绿色
    颜色单字母: b=blue g=green r=red c=cyan m=magenta y=yellow k=black w=white
    (注意 black 用 'k' 而非 'b', 'b' 表示 blue, 避免与 blue 首字母冲突)
    显式关键字参数优先于 fmt 中的同名设置。

    关键字参数 (matplotlib 兼容别名):
        lw / linewidth: 线宽 (float)
        c / color: 颜色 (如 'red', '#FF0000')
        ls / linestyle: 线型 ('-', '--', ':', '-.')
        marker: 数据点标记 ('o', 's', '^', 'D', '*', 'x', '+')
        ms / markersize: 标记大小 (float)
        mfc / markerfacecolor: 标记内部填充色
        mec / markeredgecolor: 标记边框色
        solid_capstyle: 端点 ('butt', 'round', 'projecting')
        label: 图例标签

    Returns:
        (Figure, Axes) 元组
    """
    _map_aliases(kwargs)
    pairs, _ = _parse_plot_args(args, kwargs)

    def _call(*a, **k):
        return _rsplotlib.plot(*a, **k)

    result = None
    for x, y, fmt in pairs:
        call_kwargs = dict(kwargs)
        if fmt:
            # fmt 解析出的样式作为默认值, 不覆盖用户显式传入的关键字参数
            for key, value in _parse_fmt(fmt).items():
                call_kwargs.setdefault(key, value)
        result = _route_to_ax('plot', _call, x, y, **call_kwargs)
    return result


def scatter(x, y, s=None, c=None, marker=None, cmap=None, norm=None,
            vmin=None, vmax=None, alpha=None, linewidths=None,
            edgecolors=None, plotnonfinite=False, data=None, **kwargs):
    """绘制散点图，兼容 matplotlib.pyplot.scatter 的参数签名。

    用法:
        plt.scatter(x, y)                              # 默认大小 20、默认蓝色
        plt.scatter(x, y, s=50, c='red')               # 统一大小和颜色
        plt.scatter(x, y, s=[10, 20, 30], c=['r','g','b'])   # 逐点大小/颜色
        plt.scatter(x, y, c=values, cmap='viridis')    # 数值经 colormap 映射
        plt.scatter(x, y, c=[[1,0,0],[0,1,0]])         # RGB(A) 二维行数组

    Args:
        x, y: 长度相同的数据点坐标 (list / tuple / numpy array)
        s: 点大小, 默认 20; 可为标量或与点数等长的数组
        c: 颜色; 默认蓝色; 可为颜色字符串、颜色字符串数组、数值数组
           (配合 cmap) 或 RGB(A) 二维行数组
        marker: 标记形状, 默认 'o'
        cmap: 当 c 为数值数组时使用的 colormap 名称 (如 'viridis')
        vmin, vmax: colormap 归一化范围
        alpha: 透明度 (0.0 - 1.0)
        norm / linewidths / edgecolors / plotnonfinite / data: 接受但当前不生效
        **kwargs: 额外关键字参数 (color 将作为 c 的别名)
    """
    kwargs['cmap'] = cmap
    kwargs['vmin'] = vmin
    kwargs['vmax'] = vmax
    a = 1.0 if alpha is None else alpha
    use_multi, args, mappable = _normalize_scatter(x, y, s, c, marker, label=None, alpha=a, kwargs=kwargs)
    if use_multi:
        result = _route_to_ax('scatter_multi', _rsplotlib.scatter_multi, *args)
    else:
        result = _route_to_ax('scatter', _rsplotlib.scatter, *args)
    if mappable is not None:
        ax = _get_axes()
        if ax is not None and hasattr(ax, 'set_mappable'):
            ax.set_mappable(*mappable)
    return result


def bar(x, height, width=0.8, color=None, label=None):
    """绘制柱状图。

    Args:
        x: 每个柱子的 x 坐标 (list / tuple / numpy array)
        height: 每个柱子的高度 (y 值)
        width: 柱子的宽度 (默认 0.8)
        color: 柱子的颜色字符串
        label: 图例标签

    用法:
        plt.bar([0, 1, 2], [1, 2, 3])
        plt.bar(["A", "B", "C"], [1, 2, 3])  # 字符串 x 作为类别标签
    """
    x = _to_list(x)
    height = _to_list(height)
    # 类别型 x：x 为字符串序列时，柱子落在 0,1,2,... 位置，字符串作为 x 轴刻度标签。
    tick_labels = None
    if isinstance(x, (list, tuple)) and any(isinstance(v, str) for v in x):
        tick_labels = [str(v) for v in x]
        x = list(range(len(x)))
    result = _route_to_ax('bar', _rsplotlib.bar, x, height, width, color, label)
    if tick_labels is not None:
        xticks(x, tick_labels)
    return result


def barh(y, width, height=0.8, color=None, label=None):
    """绘制水平柱状图。

    Args:
        y: 每个柱子的 y 坐标
        width: 每个柱子的宽度 (x 方向长度)
        height: 柱子的高度 (y 方向, 默认 0.8)
        color: 颜色
        label: 图例标签
    """
    y = _to_list(y)
    width = _to_list(width)
    # 类别型 y：y 为字符串序列时，柱子落在 0,1,2,... 位置，字符串作为 y 轴刻度标签。
    tick_labels = None
    if isinstance(y, (list, tuple)) and any(isinstance(v, str) for v in y):
        tick_labels = [str(v) for v in y]
        y = list(range(len(y)))
    result = _route_to_ax('barh', _rsplotlib.barh, y, width, height, color, label)
    if tick_labels is not None:
        yticks(y, tick_labels)
    return result


def hist(x, bins=10, density=False, label=None, alpha=0.7, color=None, **kwargs):
    """绘制直方图。

    用法:
        plt.hist(data, bins=20)
        plt.hist([data1, data2], bins=10, color=['red', 'blue'])

    Args:
        x: 数据 (一维数组, 或多组数据组成的列表)
        bins: 分箱数量 (默认 10)
        density: 是否归一化到概率密度 (默认 False)
        label: 图例标签
        alpha: 透明度 (默认 0.7)
        color: 颜色或颜色列表
        **kwargs: 额外关键字参数 (facecolor, align, histtype)
    """
    facecolor = kwargs.pop('facecolor', None)
    align = kwargs.pop('align', None)
    histtype = kwargs.pop('histtype', None)
    _color = facecolor if facecolor is not None else color

    x = _to_list_recursive(x)
    if x and isinstance(x[0], (list, tuple)):
        x_list = [list(v) for v in x]
    else:
        x_list = [list(x)]

    if _color is not None:
        if isinstance(_color, str):
            color_list = [_color] * len(x_list)
        elif isinstance(_color, (list, tuple)):
            color_list = list(_color)
        else:
            color_list = None
    else:
        color_list = None

    def _call_hist(*a, **k):
        return _rsplotlib.hist(*a, **k)

    return _route_to_ax('hist', _call_hist, x_list, bins=bins, density=density,
                        label=label, alpha=alpha, color=color_list,
                        facecolor=None, align=align, histtype=histtype)


def pie(x, labels=None, colors=None, autopct=False, **kwargs):
    """绘制饼图。

    用法:
        plt.pie([30, 40, 30], labels=['A', 'B', 'C'])

    Args:
        x: 数据列表 (各部分数值)
        labels: 每部分的标签列表
        colors: 每部分的颜色列表
        autopct: 百分比格式字符串 (如 '%1.1f%%'), 或布尔值 True
        **kwargs: 其他关键字参数
    """
    x = _to_list(x)
    if autopct and isinstance(autopct, str):
        autopct_str = autopct
    elif autopct:
        autopct_str = "%1.1f%%"
    else:
        autopct_str = None
    return _route_to_ax('pie', _rsplotlib.pie, x, labels, colors, autopct_str)


def boxplot(x, labels=None, vert=True, **kwargs):
    """绘制箱线图 (box-and-whisker plot)。

    展示数据的中位数、四分位、离群值等统计信息。

    用法:
        plt.boxplot([data1, data2, data3])

    Args:
        x: 数据集 (可以是一个或多个一维数组)
        labels: 每个箱的标签列表
        vert: 是否垂直绘制 (默认 True)
    """
    x = _to_list_recursive(x)
    return _route_to_ax('boxplot', _rsplotlib.boxplot, x, labels, vert)


def fill_between(x, y1, y2=0.0, color=None, alpha=0.3, label=None, **kwargs):
    """填充两条曲线之间的区域。

    用法:
        plt.fill_between(x, y1, y2, color='red', alpha=0.3)

    Args:
        x: x 坐标数据
        y1: 第一条曲线的 y 坐标
        y2: 第二条曲线的 y 坐标 (默认 0.0)
        color: 填充颜色
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        label: 图例标签
    """
    x = _to_list(x)
    y1 = _to_list(y1)
    y2 = _to_list(y2)
    return _route_to_ax('fill_between', _rsplotlib.fill_between, x, y1, y2, color, alpha, label)


def errorbar(x, y, yerr=None, xerr=None, fmt='o', color=None, label=None, capsize=3.0, **kwargs):
    """绘制带误差棒的图。

    用法:
        plt.errorbar(x, y, yerr=0.1)

    Args:
        x: x 坐标数据
        y: y 坐标数据
        yerr: y 方向误差 (标量或数组)
        xerr: x 方向误差 (标量或数组)
        fmt: 数据点/线格式字符串 (默认 'o')
        color: 颜色
        label: 图例标签
        capsize: 误差棒末端横线长度 (默认 3.0)
    """
    x = _to_list(x)
    y = _to_list(y)
    yerr = _to_list(yerr)
    xerr = _to_list(xerr)
    return _route_to_ax('errorbar', _rsplotlib.errorbar, x, y, yerr, xerr, fmt, color, label, capsize)


def stem(x, y, linefmt=None, markerfmt=None, label=None, **kwargs):
    """绘制茎叶图 (火柴杆图)。

    Args:
        x: x 坐标数据
        y: y 坐标数据
        linefmt: 线样式
        markerfmt: 标记样式
        label: 图例标签
    """
    x = _to_list(x)
    y = _to_list(y)
    return _route_to_ax('stem', _rsplotlib.stem, x, y, linefmt or '-', markerfmt or 'o', label)


def step(x, y, where='pre', label=None, color=None, linestyle='-', linewidth=1.5, **kwargs):
    """绘制阶梯图。

    Args:
        x: x 坐标数据
        y: y 坐标数据
        where: 阶梯位置 ('pre', 'mid', 'post', 默认 'pre')
        label: 图例标签
        color: 颜色
        linestyle: 线型
        linewidth: 线宽
    """
    x = _to_list(x)
    y = _to_list(y)
    return _route_to_ax('step', _rsplotlib.step, x, y, where, label, color, linestyle, linewidth)


def stackplot(x, *args, labels=None, colors=None, alpha=1.0, **kwargs):
    """绘制堆叠面积图。

    用法:
        plt.stackplot(x, y1, y2, y3, labels=['A', 'B', 'C'])

    Args:
        x: x 坐标数据
        *args: 多个 y 数据集
        labels: 每个数据集的标签列表
        colors: 每个数据集的颜色列表
        alpha: 透明度 (默认 1.0)
    """
    x = _to_list(x)
    y_data = [list(a) for a in args if a is not None]
    if not y_data and 'y' in kwargs:
        y_data = [_to_list(kwargs['y'])]
    return _route_to_ax('stackplot', _rsplotlib.stackplot, x, *y_data,
                        labels=labels, colors=colors, alpha=alpha)


def imshow(x, cmap='viridis', aspect='auto', **kwargs):
    """显示图像 (矩阵热力图)。

    Args:
        x: 2D 数组 (行对应 y 轴, 列对应 x 轴)
        cmap: 颜色映射名称 (默认 'viridis')
        aspect: 宽高比 ('auto', 'equal', 或数值)
    """
    x = _to_list_recursive(x)
    return _route_to_ax('imshow', _rsplotlib.imshow, x, cmap, aspect)


def semilogx(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 x 轴对数刻度图。"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.semilogx(x, y, label, color, linestyle, marker, linewidth)


def semilogy(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 y 轴对数刻度图。"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.semilogy(x, y, label, color, linestyle, marker, linewidth)


def loglog(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制双对数刻度图。"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.loglog(x, y, label, color, linestyle, marker, linewidth)


# ==================== 辅助元素 ====================

def text(x, y, s, fontdict=None, **kwargs):
    """添加文本标注。

    Args:
        x, y: 文本位置 (数据坐标)
        s: 文本内容
        fontdict: 字体属性字典 (可选)
        **kwargs: 支持 fontsize, color/c, family 等参数
    """
    fontsize = kwargs.get('fontsize', fontdict.get('fontsize', 12) if fontdict else 12)
    color = kwargs.get('color', fontdict.get('color', 'black') if fontdict else 'black')
    c = kwargs.get('c', None)
    family = kwargs.get('family', None)
    if not isinstance(s, str):
        s = str(s)

    # family 处理：若用户显式指定了字体族名，解析为本地字体文件并注册到
    # plotters 的字体数据库，这样真正驱动文本渲染（而不是被忽略）。
    if family:
        try:
            import os
            from .utils._font_resolver import resolve_font_path
            path = resolve_font_path(family)
            if path is None and os.path.isfile(family):
                path = family  # 也允许直接传文件路径
            if path is not None:
                _rsplotlib.register_sans_serif_font(path)
        except Exception:
            # 字体注册失败不影响绘制（会回退到默认 sans-serif）
            pass

    return _rsplotlib.text(x, y, s, fontsize, color, c, family)


def axhline(y=0, **kwargs):
    """绘制水平参考线。

    Args:
        y: y 坐标位置
        color: 线颜色
        linestyle: 线型 ('-', '--', ':', '-.')
        linewidth: 线宽
    """
    return _rsplotlib.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axvline(x=0, **kwargs):
    """绘制垂直参考线。

    Args:
        x: x 坐标位置
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
    """
    return _rsplotlib.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axhspan(ymin, ymax, **kwargs):
    """绘制水平区间填充 (在 y 方向高亮一个区间)。

    用法:
        plt.axhspan(0.0, 1.0, color='yellow', alpha=0.3)

    Args:
        ymin: y 轴下限
        ymax: y 轴上限
        color: 填充颜色 (默认蓝灰色)
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        **kwargs: 其他关键字参数
    """
    return _rsplotlib.axhspan(ymin, ymax, kwargs.get('color'), kwargs.get('alpha', 0.3))


def axvspan(xmin, xmax, **kwargs):
    """绘制垂直区间填充 (在 x 方向高亮一个区间)。

    用法:
        plt.axvspan(0.0, 1.0, color='yellow', alpha=0.3)

    Args:
        xmin: x 轴下限
        xmax: x 轴上限
        color: 填充颜色 (默认蓝灰色)
        alpha: 透明度 (0.0-1.0, 默认 0.3)
        **kwargs: 其他关键字参数
    """
    return _rsplotlib.axvspan(xmin, xmax, kwargs.get('color'), kwargs.get('alpha', 0.3))


def axline(xy1, xy2, **kwargs):
    """通过两点绘制任意斜率的直线 (延长到整个绘图区域)。

    用法:
        plt.axline((0, 0), (1, 1), color='red')

    Args:
        xy1: 起点坐标 (x1, y1)
        xy2: 终点坐标 (x2, y2)
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    return _rsplotlib.axline(
        tuple(xy1), tuple(xy2),
        kwargs.get('color'), kwargs.get('linestyle'),
        kwargs.get('linewidth'),
    )


def annotate(text, xy, xytext=None, fontsize=12.0, color='black', arrowprops=None, **kwargs):
    """在指定坐标添加文本标注, 可选带箭头。

    用法:
        plt.annotate('重要点', xy=(1, 2), xytext=(3, 4),
                     arrowprops=dict(arrowstyle='->'))

    Args:
        text: 标注文本内容
        xy: 被标注点的坐标 (数据坐标)
        xytext: 文本放置位置 (数据坐标)。若提供, 自动从该位置绘制箭头到 xy
        fontsize: 字体大小 (默认 12.0)
        color: 文本和箭头颜色
        arrowprops: 箭头属性字典 (支持 arrowstyle, arrowsize 等)
        **kwargs: 其他关键字参数
    """
    arrowstyle = None
    arrowsize = 1.0
    if arrowprops is not None:
        if isinstance(arrowprops, dict):
            if 'arrowstyle' in arrowprops:
                arrowstyle = arrowprops['arrowstyle']
            if 'arrowsize' in arrowprops:
                arrowsize = arrowprops['arrowsize']
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'annotate'):
        ax.annotate(text, xy, xytext, fontsize, color, arrowprops, arrowstyle, arrowsize)
        return _get_figure()
    fig, ax = _rsplotlib.subplots()
    ax.annotate(text, xy, xytext, fontsize, color, arrowprops, arrowstyle, arrowsize)
    return fig, ax


def hlines(y, xmin=None, xmax=None, **kwargs):
    """在指定 y 位置绘制多条水平线段。

    由 Rust 层批量实现, 避免 Python 级 for 循环。

    Args:
        y: 单个 y 值 或 多个 y 值的列表
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    y_arr = _to_list(y) if isinstance(y, (list, tuple)) or hasattr(y, 'tolist') else [y]
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'hlines'):
        ax.hlines(y_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))
        return _get_figure()
    return _rsplotlib.hlines(y_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def vlines(x, ymin=None, ymax=None, **kwargs):
    """在指定 x 位置绘制多条垂直线段 (Rust 层批量实现)。

    Args:
        x: 单个 x 值 或 多个 x 值的列表
        color: 线颜色
        linestyle: 线型
        linewidth: 线宽
        **kwargs: 其他关键字参数
    """
    x_arr = _to_list(x) if isinstance(x, (list, tuple)) or hasattr(x, 'tolist') else [x]
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'vlines'):
        ax.vlines(x_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))
        return _get_figure()
    return _rsplotlib.vlines(x_arr, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


# ==================== 配置函数 ====================

def _font_props(fontdict, kwargs):
    """从 fontdict 与关键字参数中提取字体属性 (family, size, color)。

    matplotlib 语义：关键字参数优先于 fontdict；family 支持 family/fontfamily/fontname
    别名，size 支持 size/fontsize，color 支持 color/c。返回 (family, size, color)，
    其中 size 已转为 float 或 None。
    """
    fd = fontdict or {}

    def _pick(*keys):
        for k in keys:
            if kwargs.get(k) is not None:
                return kwargs[k]
        for k in keys:
            if fd.get(k) is not None:
                return fd[k]
        return None

    family = _pick('family', 'fontfamily', 'fontname')
    size = _pick('size', 'fontsize')
    color = _pick('color', 'c')
    size = float(size) if size is not None else None
    return family, size, color


def xlabel(text, fontdict=None, loc=None, **kwargs):
    """设置 x 轴标签文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性 (fontdict 的键或直接关键字参数, 关键字参数优先):
        family / fontfamily / fontname: 字体族名 (如 'Courier'、'STHeiti Light' 等)
        size / fontsize: 字号 (points)
        color: 文本颜色 (如 'r'、'#ff0000'、'SeaGreen')

    loc: 标签水平位置，可选 'left'、'center'、'right'，默认 'center'。

    用法:
        plt.xlabel("x - label")
        plt.xlabel("x 轴", fontdict={"family": "STHeiti Light", "size": 16, "color": "b"})
        plt.xlabel("x 轴", loc="left")
    """
    family, size, color = _font_props(fontdict, kwargs)
    return _rsplotlib.xlabel(text, color, size, family, loc)


def ylabel(text, fontdict=None, loc=None, **kwargs):
    """设置 y 轴标签文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性同 xlabel（family / size / color，关键字参数优先于 fontdict）。

    loc: 标签垂直位置，可选 'bottom'、'center'、'top'，默认 'center'。

    用法:
        plt.ylabel("y - label")
        plt.ylabel("y 轴", fontdict={"family": "STHeiti Light", "size": 16, "color": "g"})
        plt.ylabel("y 轴", loc="top")
    """
    family, size, color = _font_props(fontdict, kwargs)
    return _rsplotlib.ylabel(text, color, size, family, loc)


def title(label, fontdict=None, loc=None, **kwargs):
    """设置图表标题文本，并可通过 fontdict / 关键字参数自定义字体属性。

    支持的字体属性 (fontdict 的键或直接关键字参数, 关键字参数优先):
        family / fontfamily / fontname: 字体族名 (如 'Courier'、'Times New Roman'、
            'SimHei' 等)
        size / fontsize: 字号 (points)
        color: 文本颜色 (如 'r'、'#ff0000'、'SeaGreen')

    loc: 标题水平位置，可选 'left'、'center'、'right'，默认 'center'。

    用法:
        plt.title("标题")
        plt.title("标题", fontdict={"family": "Courier", "size": 18, "color": "red"})
        plt.title("标题", fontsize=18, color='b')
        plt.title("标题", loc="left")
    """
    family, size, color = _font_props(fontdict, kwargs)
    # 字体族名的解析与注册由 Rust 层的 set_title 统一处理。
    return _rsplotlib.title(label, color, size, family, loc)


def suptitle(t, **kwargs):
    """设置整个图形的总标题（居中显示在所有子图上方）。

    Args:
        t: 标题文本

    用法:
        plt.suptitle("总标题")
    """
    return _rsplotlib.gcf().suptitle(str(t))


def grid(visible=True, **kwargs):
    """显示或隐藏网格线。

    Args:
        visible: 是否显示 (默认 True)
        color/c: 线颜色
        linestyle/ls: 线型
        linewidth/lw: 线宽
        axis: 坐标轴 ('x', 'y', 或 'both')
    """
    c = kwargs.get('color') or kwargs.get('c')
    ls = kwargs.get('linestyle') or kwargs.get('ls')
    lw = kwargs.get('linewidth') or kwargs.get('lw')
    axis = kwargs.get('axis')
    return _rsplotlib.grid(visible, c, ls, lw, axis)


def legend(loc='best', **kwargs):
    """显示图例 (需要 plot 时设置 label 参数)。

    Args:
        loc: 图例位置 ('best', 'upper right', 'upper left', 'lower left',
              'lower right', 'upper center', 'lower center',
              'center left', 'center right', 'center')
    """
    return _rsplotlib.legend(loc)


def xlim(left=None, right=None, **kwargs):
    """设置 x 轴显示范围。

    Args:
        left: x 轴最小值
        right: x 轴最大值
    """
    return _rsplotlib.xlim(left, right)


def ylim(bottom=None, top=None, **kwargs):
    """设置 y 轴显示范围。

    Args:
        bottom: y 轴最小值
        top: y 轴最大值
    """
    return _rsplotlib.ylim(bottom, top)


def xticks(ticks=None, labels=None, **kwargs):
    """设置 x 轴刻度。

    Args:
        ticks: 刻度位置列表
        labels: 刻度标签列表
    """
    ticks = _to_list(ticks)
    return _rsplotlib.xticks(ticks, labels)


def yticks(ticks=None, labels=None, **kwargs):
    """设置 y 轴刻度。

    Args:
        ticks: 刻度位置列表
        labels: 刻度标签列表
    """
    ticks = _to_list(ticks)
    return _rsplotlib.yticks(ticks, labels)


def xscale(scale, **kwargs):
    """设置 x 轴刻度类型 ('linear', 'log', 'logit', 'symlog' 等)。"""
    return _rsplotlib.xscale(scale)


def yscale(scale, **kwargs):
    """设置 y 轴刻度类型。"""
    return _rsplotlib.yscale(scale)


def margins(x_margin=None, y_margin=None, **kwargs):
    """设置自动缩放的边距。"""
    return _rsplotlib.margins(x_margin, y_margin)


def box(on=None):
    """设置坐标轴边框。"""
    return _rsplotlib.box_(on)


def minorticks_on():
    """显示次要刻度。"""
    return _rsplotlib.minorticks_on()


def minorticks_off():
    """隐藏次要刻度。"""
    return _rsplotlib.minorticks_off()


# ==================== 子图与布局 ====================

def subplots(nrows=1, ncols=1, figsize=None, dpi=None, squeeze=True, **kwargs):
    """创建子图网格 (Figure + Axes)。

    用法:
        fig, ax = plt.subplots()                    # 单图 (1x1)
        fig, axes = plt.subplots(2, 2)             # 2x2 网格
        fig, axes = plt.subplots(1, 2, figsize=(10, 5))  # 自定义尺寸

    Args:
        nrows: 子图行数 (默认 1)
        ncols: 子图列数 (默认 1)
        figsize: 图的尺寸 (width, height), 单位英寸
        dpi: 分辨率 (每英寸点数)
        squeeze: 是否压缩返回的 Axes 数组维度 (默认 True), 与 matplotlib 一致
        **kwargs: 其他关键字参数

    Returns:
        与 matplotlib 一致的返回值 (squeeze=True 时):
        - 1x1: (Figure, Axes)
        - 1xN 或 Nx1: (Figure, 一维 ndarray[Axes])
        - MxN: (Figure, 二维 ndarray[Axes]), 支持 axs[i, j] 索引
    """
    result = _rsplotlib.subplots(nrows, ncols, figsize, dpi)
    fig = result[0]

    if nrows == 1 and ncols == 1:
        single = result[1]
        if squeeze:
            return fig, single
        flat = [single]
    else:
        flat = list(result[1])

    try:
        import numpy as np
    except ImportError:
        # numpy 不可用时降级为嵌套 list（不支持 axs[i, j] 元组索引）。
        # matplotlib 本身依赖 numpy，多子图场景下建议安装 numpy。
        rows = [[flat[r * ncols + c] for c in range(ncols)] for r in range(nrows)]
        if squeeze:
            if nrows == 1:
                return fig, rows[0]
            if ncols == 1:
                return fig, [row[0] for row in rows]
        return fig, rows

    axarr = np.empty((nrows, ncols), dtype=object)
    for r in range(nrows):
        for c in range(ncols):
            axarr[r, c] = flat[r * ncols + c]
    if squeeze:
        axarr = axarr.squeeze()
        if axarr.ndim == 0:
            return fig, axarr.item()
    return fig, axarr


def subplot(nrows, ncols, index, **kwargs):
    """创建单个子图。"""
    return _rsplotlib.subplot(nrows, ncols, index)


def tight_layout(**kwargs):
    """自动调整子图布局, 避免标签重叠。"""
    return _rsplotlib.tight_layout()


def subplots_adjust(left=None, right=None, bottom=None, top=None, wspace=None, hspace=None):
    """调整子图布局参数"""
    fig = _get_figure()
    if fig is not None:
        fig.subplots_adjust(left, right, bottom, top, wspace, hspace)


def set_size(width, height):
    """设置图形像素尺寸。"""
    return _rsplotlib.set_size(width, height)


def twinx():
    """创建共享 x 轴的双 y 轴。"""
    return _rsplotlib.twinx()


def twiny():
    """创建共享 y 轴的双 x 轴。"""
    return _rsplotlib.twiny()


# ==================== 图形控制 ====================

def figure(num=None, figsize=None, dpi=None, **kwargs):
    """创建新的 Figure 对象。

    Args:
        num: 图形编号 (兼容 matplotlib, 未实际使用)
        figsize: (width, height) 英寸数
        dpi: 分辨率
        **kwargs: 其他关键字参数

    Returns:
        Figure 对象
    """
    fig = _rsplotlib.figure()
    d = dpi if dpi is not None else DEFAULT_DPI
    fig.set_dpi(d)
    if figsize is not None:
        w_inch, h_inch = figsize
        fig.set_size(round(w_inch * d), round(h_inch * d))
    else:
        w, h = _get_rcparams().get('figure.figsize', list(DEFAULT_FIGSIZE))
        fig.set_size(round(w * d), round(h * d))
    return fig


def savefig(fname, **kwargs):
    """保存图形到文件。

    支持的文件格式:
        - .png: 便携式网络图形 (位图)
        - .jpg: JPEG 图像
        - .svg: 可缩放矢量图形

    用法:
        plt.savefig('figure.png')
        plt.savefig('figure.png', dpi=300)   # 高分辨率
        plt.savefig('figure.svg')

    Args:
        fname: 输出文件名 (含扩展名)
        dpi: 分辨率 (每英寸点数, 默认与创建时一致)。
             对于高清晰度出版物, 使用 300 或更高。
        **kwargs: 其他关键字参数
    """
    dpi = kwargs.get('dpi')
    fig = _get_figure()
    if fig is not None:
        if dpi is not None:
            fig.savefig(fname, dpi)
        else:
            fig.savefig(fname, dpi=DEFAULT_DPI)
        return
    # 无 Figure 时回退到模块级
    if dpi is not None:
        _rsplotlib.savefig(fname, dpi)
    else:
        _rsplotlib.savefig(fname)


def show(**kwargs):
    """在默认应用中显示图形。无当前 figure 时静默返回（与 matplotlib 一致）。"""
    if _get_figure() is None:
        return None
    return _rsplotlib.show()


def gca(**kwargs):
    """获取当前 Axes。"""
    return _rsplotlib.gca()


def gcf(**kwargs):
    """获取当前 Figure。"""
    return _rsplotlib.gcf()


def cla():
    """清空当前 Axes 内容。"""
    return _rsplotlib.cla()


def clf():
    """清空当前 Figure 内容 (清除所有子图)。"""
    return _rsplotlib.clf()


def close(fig=None):
    """关闭当前 Figure。

    Args:
        fig: 图形或 'all' (兼容 matplotlib)
    """
    return _rsplotlib.close()


def axis(arg=None, **kwargs):
    """坐标轴控制: axis('off') 隐藏, axis('equal') 等比例。"""
    if arg == 'off':
        try:
            _rsplotlib.gca()._axis_off()
        except Exception:
            pass
    elif arg in ('equal', 'scaled'):
        gca().set_aspect('equal')
    return None


def colorbar(mappable=None, **kwargs):
    """在当前坐标区右侧添加颜色条。

    颜色条基于最近一次可映射绘制（scatter 数值 c + cmap，或 imshow）记录的
    (cmap, vmin, vmax) 信息渲染。若此前没有可映射绘制，则按 viridis / [0,1] 兜底。
    """
    ax = _get_axes()
    if ax is not None and hasattr(ax, 'enable_colorbar'):
        ax.enable_colorbar()
    return None


def get_cmap(name=None, lut=None):
    """获取颜色映射 (占位实现)。"""
    return name


# ==================== Figure 类补丁 ====================

def _patch_figure_add_subplot():
    """为 Rust Figure 类添加 add_subplot(nrows, ncols, index) 支持。"""
    from . import rsplotlib as _rs

    _orig_add_subplot = _rs.Figure.add_subplot

    def _add_subplot(self, *args):
        if len(args) == 1:
            return _orig_add_subplot(self, args[0])
        elif len(args) == 3:
            nrows, ncols, index = args
            if isinstance(index, tuple):
                indices = [i - 1 for i in index]
                row_start = indices[0] // ncols
                row_end = indices[-1] // ncols + 1
                col_start = indices[0] % ncols
                col_end = indices[-1] % ncols + 1
            else:
                index_0 = index - 1
                row_start = index_0 // ncols
                row_end = row_start + 1
                col_start = index_0 % ncols
                col_end = col_start + 1
            from .layout.gridspec import SubplotSpec
            spec = SubplotSpec(None, row_start, row_end, col_start, col_end)
            return _orig_add_subplot(self, spec)
        else:
            raise TypeError(
                f"add_subplot() takes 1 or 3 positional arguments but {len(args)} were given"
            )

    _rs.Figure.add_subplot = _add_subplot


def _patch_axes():
    """为 Rust Axes 类添加 Python 级别的 API 兼容补丁。"""
    from . import rsplotlib as _rs

    # plot: 支持单参数 ax.plot(y)
    _orig_plot = _rs.Axes.plot

    def _plot(self, *args, **kwargs):
        if len(args) == 1:
            y = args[0]
            x = list(range(len(y) if hasattr(y, '__len__') else 0))
            return _orig_plot(self, x, y, **kwargs)
        return _orig_plot(self, *args, **kwargs)

    _rs.Axes.plot = _plot

    # hlines / vlines: Rust 层已支持批量, 直接转发
    _orig_hlines = _rs.Axes.hlines
    _orig_vlines = _rs.Axes.vlines

    def _hlines(self, y, xmin=None, xmax=None, color=None, linestyle=None, linewidth=None, **kwargs):
        y_arr = _to_list(y) if isinstance(y, (list, tuple)) or hasattr(y, 'tolist') else [y]
        return _orig_hlines(self, y_arr, color, linestyle, linewidth)

    def _vlines(self, x, ymin=None, ymax=None, color=None, linestyle=None, linewidth=None, **kwargs):
        x_arr = _to_list(x) if isinstance(x, (list, tuple)) or hasattr(x, 'tolist') else [x]
        return _orig_vlines(self, x_arr, color, linestyle, linewidth)

    _rs.Axes.hlines = _hlines
    _rs.Axes.vlines = _vlines

    # scatter: 支持 c/s 为数组、数值 c + cmap、RGB(A) 二维行数组
    _orig_scatter = _rs.Axes.scatter

    def _scatter(self, x, y, s=None, c=None, marker=None, label=None, alpha=1.0, **kwargs):
        use_multi, args, mappable = _normalize_scatter(x, y, s, c, marker, label, alpha, kwargs)
        if mappable is not None:
            self.set_mappable(*mappable)
        if use_multi:
            return self.scatter_multi(*args)
        return _orig_scatter(self, *args)

    _rs.Axes.scatter = _scatter

    # set_xlim / set_ylim: 支持元组参数 (left, right)
    _orig_set_xlim = _rs.Axes.set_xlim
    _orig_set_ylim = _rs.Axes.set_ylim

    def _set_xlim(self, *args, **kwargs):
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            lo, hi = args[0]
            return _orig_set_xlim(self, left=lo, right=hi)
        return _orig_set_xlim(self, *args, **kwargs)

    def _set_ylim(self, *args, **kwargs):
        if len(args) == 1 and isinstance(args[0], (tuple, list)):
            lo, hi = args[0]
            return _orig_set_ylim(self, bottom=lo, top=hi)
        return _orig_set_ylim(self, *args, **kwargs)

    _rs.Axes.set_xlim = _set_xlim
    _rs.Axes.set_ylim = _set_ylim

    # set(**kwargs): matplotlib 语义, 每个 key 映射到 set_<key>(value)
    def _ax_set(self, **kwargs):
        for key, value in kwargs.items():
            setter = getattr(self, 'set_' + key, None)
            if setter is None:
                continue
            # numpy/rsnumpy 数组转 list, 供 Rust 侧 Vec<f64> 提取; tuple 保留给 set_xlim 处理
            if hasattr(value, 'tolist'):
                value = value.tolist()
            setter(value)
        return None

    _rs.Axes.set = _ax_set


_patch_figure_add_subplot()
_patch_axes()


style = _style_module.style
