"""rsplotlib.pyplot - Matplotlib pyplot 兼容接口

此模块提供与 matplotlib.pyplot 兼容的 API，所有函数代理到 rsplotlib 核心模块。
使用方法: import rsplotlib.pyplot as plt
"""

from . import rsplotlib as _rsplotlib
from ._figure_defaults import DEFAULT_DPI, DEFAULT_FIGSIZE
# ============ 样式接口 ============
from . import style as _style_module

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


# ==================== 绘图函数 ====================

def plot(*args, **kwargs):
    """绘制折线图。

    用法:
        plt.plot(x, y)              # 以 x 为横坐标, y 为纵坐标
        plt.plot(y)                 # 仅提供 y, 自动 x = [0, 1, ...]
        plt.plot(x, y, lw=2.0)     # 自定义线宽
        plt.plot(x1, y1, x2, y2)   # 绘制多条线

    关键字参数 (matplotlib 兼容别名):
        lw / linewidth: 线宽 (float)
        c / color: 颜色 (如 'red', '#FF0000')
        ls / linestyle: 线型 ('-', '--', ':', '-.')
        marker: 数据点标记 ('o', 's', '^', 'D', '*', 'x', '+')
        solid_capstyle: 端点 ('butt', 'round', 'projecting')
        label: 图例标签

    Returns:
        (Figure, Axes) 元组
    """
    # 别名、fmt 解析、调用模式检测全部由 Rust 层处理
    if len(args) >= 4 and len(args) % 2 == 0:
        results = []
        for i in range(0, len(args), 2):
            results.append(_route_to_ax('plot', _rsplotlib.plot, args[i], args[i+1], **kwargs))
        return results if len(results) > 1 else results[0]

    return _route_to_ax('plot', _rsplotlib.plot, *args, **kwargs)


def scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, **kwargs):
    """绘制散点图。

    支持每个点独立的颜色和大小:
        plt.scatter(x, y, s=50, c='red')          # 统一大小和颜色
        plt.scatter(x, y, s=[10, 20, 30], c=['red', 'green', 'blue'])

    Args:
        x: x 坐标 (list / tuple / numpy array)
        y: y 坐标 (list / tuple / numpy array)
        s: 点大小, 单个浮点数 或 浮点数数组
        c: 颜色, 单个字符串 或 颜色字符串数组
        marker: 标记形状 ('o', 's', '^', 'D', '*', 'x', '+')
        label: 图例标签
        alpha: 透明度 (0.0 - 1.0)
        **kwargs: 额外关键字参数 (color 将作为 c 的别名)
    """
    x = _to_list(x)
    y = _to_list(y)
    # 支持 color 作为 c 的别名
    if c is None and 'color' in kwargs:
        c = kwargs.pop('color')
    # 如果 s 或 c 是数组, 则路由到 scatter_multi (Rust 层批量实现)
    if isinstance(s, (list, tuple)) or (isinstance(c, (list, tuple)) and c and isinstance(c[0], str)):
        return _route_to_ax('scatter_multi', _rsplotlib.scatter_multi, x, y, s, c, marker, label, alpha)
    return _route_to_ax('scatter', _rsplotlib.scatter, x, y, s, c, marker, label, alpha)


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
    """
    x = _to_list(x)
    height = _to_list(height)
    return _route_to_ax('bar', _rsplotlib.bar, x, height, width, color, label)


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
    return _route_to_ax('barh', _rsplotlib.barh, y, width, height, color, label)


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
            from ._font_resolver import resolve_font_path
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
    return _rsplotlib.axline(tuple(xy1), tuple(xy2), kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


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

def xlabel(text, **kwargs):
    """设置 x 轴标签文本。"""
    return _rsplotlib.xlabel(text)


def ylabel(text, **kwargs):
    """设置 y 轴标签文本。"""
    return _rsplotlib.ylabel(text)


def title(label, fontdict=None, **kwargs):
    """设置图表标题文本。"""
    return _rsplotlib.title(label)


def grid(visible=True, **kwargs):
    """显示或隐藏网格线。

    Args:
        visible: 是否显示 (默认 True)
        color/c: 线颜色
        linestyle/ls: 线型
        linewidth/lw: 线宽
        axis: 坐标轴 ('x', 'y', 或 'both')
    """
    c = kwargs.get('c')
    ls = kwargs.get('linestyle') or kwargs.get('ls')
    lw = kwargs.get('linewidth') or kwargs.get('lw')
    axis = kwargs.get('axis')
    return _rsplotlib.grid(visible, c, ls, lw, axis)


def legend(loc='best', **kwargs):
    """显示图例 (需要 plot 时设置 label 参数)。

    Args:
        loc: 图例位置 ('best', 'upper right', 'upper left', 'lower left', 'lower right', 'upper center', 'lower center', 'center left', 'center right', 'center')
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

def subplots(nrows=1, ncols=1, figsize=None, dpi=None, **kwargs):
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
        **kwargs: 其他关键字参数

    Returns:
        (Figure, Axes) 或 (Figure, list[Axes]) 元组
    """
    result = _rsplotlib.subplots(nrows, ncols, figsize, dpi)
    if nrows == 1 and ncols == 1:
        return result  # (fig, ax)
    fig = result[0]
    flat_axes = list(result[1])
    # 组织为 2D 列表
    axes_2d = []
    for r in range(nrows):
        row = [flat_axes[r * ncols + c] for c in range(ncols)]
        axes_2d.append(row)
    return fig, axes_2d


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
            fig.savefig(fname)
        return
    # 无 Figure 时回退到模块级
    if dpi is not None:
        _rsplotlib.savefig(fname, dpi)
    else:
        _rsplotlib.savefig(fname)


def show(**kwargs):
    """在默认应用中显示图形。
    
    图形将保存到与执行脚本相同的路径下，文件名为脚本文件名（扩展名替换为 .png）。
    例如：运行 runtest.py 会生成 runtest.png
    """
    import sys
    import os
    
    script_path = sys.argv[0] if sys.argv else 'rsplotlib_output'
    script_dir = os.path.dirname(script_path)
    script_name = os.path.basename(script_path)
    
    base_name = os.path.splitext(script_name)[0]
    output_path = os.path.join(script_dir, f'{base_name}.png')
    
    savefig(output_path, **kwargs)
    
    if sys.platform == 'darwin':
        os.system(f'open "{output_path}"')
    elif sys.platform == 'linux':
        os.system(f'xdg-open "{output_path}"')
    
    print(f"Figure saved to: {output_path}")


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
    """添加颜色条 (占位实现)。"""
    pass


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
            from .gridspec import SubplotSpec
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

    # scatter: 支持 c/s 为数组
    _orig_scatter = _rs.Axes.scatter

    def _scatter(self, x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, **kwargs):
        color = kwargs.pop('color', None)
        if c is None and color is not None:
            c = color
        is_multi_s = isinstance(s, (list, tuple))
        is_multi_c = isinstance(c, (list, tuple)) and c and isinstance(c[0], str)
        if is_multi_s or is_multi_c:
            return self.scatter_multi(x, y, s if is_multi_s else None, c if is_multi_c else None, marker, label, alpha)
        return _orig_scatter(self, x, y, s, c, marker, label, alpha)

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


_patch_figure_add_subplot()
_patch_axes()


style = _style_module.style
