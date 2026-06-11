"""rsplotlib.pyplot - Matplotlib pyplot 兼容接口

此模块提供与 matplotlib.pyplot 兼容的 API，所有函数代理到 rsplotlib 核心模块。
使用方法: import rsplotlib.pyplot as plt
"""

from . import rsplotlib as _rsplotlib
from ._rcparams import rcParams
from ._figure_defaults import DEFAULT_DPI, DEFAULT_FIGSIZE


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


def _route_to_ax(ax_method_name, module_method, *args):
    """将调用路由到当前 axes（如果存在）或模块级函数

    Args:
        ax_method_name: axes 的方法名（字符串）
        module_method: 模块级函数（可调用对象）
        *args: 传递给方法的参数
    """
    ax = _get_axes()
    if ax is not None and hasattr(ax, ax_method_name):
        method = getattr(ax, ax_method_name)
        method(*args)
        return _get_figure()
    return module_method(*args)


def _map_aliases(kwargs):
    """规范化 matplotlib 别名到标准名"""
    alias_map = {
        'lw': 'linewidth',
        'c': 'color',
        'ls': 'linestyle',
    }
    for alias, target in alias_map.items():
        if alias in kwargs and target not in kwargs:
            kwargs[target] = kwargs.pop(alias)
        elif alias in kwargs:
            kwargs.pop(alias)


def _parse_plot_args(args, kwargs):
    """解析 plot() 的位置参数为 (x, y, kwargs)"""
    if len(args) == 2:
        return args[0], args[1], kwargs
    elif len(args) == 1:
        try:
            x = list(range(len(args[0])))
        except Exception:
            x = list(args[0]) if hasattr(args[0], '__iter__') else []
        return x, args[0], kwargs
    return [], [], kwargs


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
    x = _to_list(x)
    y = _to_list(y)
    plot_args = (x, y, kw.get('label'), kw.get('color'), kw.get('linestyle'),
                 kw.get('marker'), kw.get('linewidth'))
    return _route_to_ax('plot', lambda *a: _rsplotlib.plot(*a), *plot_args)


def scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, **kwargs):
    """绘制散点图"""
    x = _to_list(x)
    y = _to_list(y)
    # 支持 color 作为 c 的别名
    if c is None and 'color' in kwargs:
        c = kwargs.pop('color')
    return _route_to_ax('scatter', _rsplotlib.scatter, x, y, s, c, marker, label, alpha)


def bar(x, height, width=0.8, color=None, label=None):
    """绘制柱状图"""
    x = _to_list(x)
    height = _to_list(height)
    return _route_to_ax('bar', _rsplotlib.bar, x, height, width, color, label)


def barh(y, width, height=0.8, color=None, label=None):
    """绘制水平柱状图"""
    y = _to_list(y)
    width = _to_list(width)
    return _route_to_ax('barh', _rsplotlib.barh, y, width, height, color, label)


def hist(x, bins=10, density=False, label=None, alpha=0.7, color=None, **kwargs):
    """绘制直方图

    支持 matplotlib 兼容的参数:
        facecolor: 填充颜色 (优先级高于 color)
        align: 对齐方式 ('mid', 'left', 'right')
        histtype: 直方图类型 ('bar', 'step', 'stepfilled')

    如果存在当前 axes，则复用它；否则创建新的 Figure 和 Axes。
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

    return _route_to_ax('hist', _rsplotlib.hist, x_list, bins, density, label, alpha, color_list, None, align, histtype)


def pie(x, labels=None, colors=None, autopct=False, **kwargs):
    """绘制饼图"""
    x = _to_list(x)
    if autopct and isinstance(autopct, str):
        autopct_str = autopct
    elif autopct:
        autopct_str = "%1.1f%%"
    else:
        autopct_str = None
    return _route_to_ax('pie', _rsplotlib.pie, x, labels, colors, autopct_str)


def boxplot(x, labels=None, vert=True, **kwargs):
    """绘制箱线图"""
    x = _to_list_recursive(x)
    return _route_to_ax('boxplot', _rsplotlib.boxplot, x, labels, vert)


def fill_between(x, y1, y2=0.0, color=None, alpha=0.3, label=None, **kwargs):
    """填充区域"""
    x = _to_list(x)
    y1 = _to_list(y1)
    y2 = _to_list(y2)
    return _route_to_ax('fill_between', _rsplotlib.fill_between, x, y1, y2, color, alpha, label)


def errorbar(x, y, yerr=None, xerr=None, fmt='o', color=None, label=None, capsize=3.0, **kwargs):
    """绘制误差棒图"""
    x = _to_list(x)
    y = _to_list(y)
    yerr = _to_list(yerr)
    xerr = _to_list(xerr)
    return _route_to_ax('errorbar', _rsplotlib.errorbar, x, y, yerr, xerr, fmt, color, label, capsize)


def stem(x, y, linefmt=None, markerfmt=None, label=None, **kwargs):
    """绘制茎叶图"""
    x = _to_list(x)
    y = _to_list(y)
    return _route_to_ax('stem', _rsplotlib.stem, x, y, linefmt or '-', markerfmt or 'o', label)


def step(x, y, where='pre', label=None, color=None, linestyle='-', linewidth=1.5, **kwargs):
    """绘制阶梯图"""
    x = _to_list(x)
    y = _to_list(y)
    return _route_to_ax('step', _rsplotlib.step, x, y, where, label, color, linestyle, linewidth)


def imshow(x, cmap='viridis', aspect='auto', **kwargs):
    """显示图像"""
    x = _to_list_recursive(x)
    return _route_to_ax('imshow', _rsplotlib.imshow, x, cmap, aspect)


def semilogx(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 x 轴对数刻度图"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.semilogx(x, y, label, color, linestyle, marker, linewidth)


def semilogy(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制 y 轴对数刻度图"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.semilogy(x, y, label, color, linestyle, marker, linewidth)


def loglog(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None, **kwargs):
    """绘制双对数刻度图"""
    x = _to_list(x)
    y = _to_list(y)
    return _rsplotlib.loglog(x, y, label, color, linestyle, marker, linewidth)


# ==================== 辅助元素 ====================

def text(x, y, s, fontdict=None, **kwargs):
    """添加文本

    Parameters
    ----------
    x, y : float
        文本位置（数据坐标）
    s : str
        文本内容
    fontdict : dict, optional
        字体属性字典（matplotlib 兼容）
    **kwargs
        fontsize, color/c, family 等 matplotlib 兼容参数
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
    """添加水平参考线"""
    return _rsplotlib.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def axvline(x=0, **kwargs):
    """添加垂直参考线"""
    return _rsplotlib.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def hlines(y, xmin, xmax, **kwargs):
    """绘制水平线段"""
    return _rsplotlib.axhline(y, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


def vlines(x, ymin, ymax, **kwargs):
    """绘制垂直线段"""
    return _rsplotlib.axvline(x, kwargs.get('color'), kwargs.get('linestyle'), kwargs.get('linewidth'))


# ==================== 配置函数 ====================

def xlabel(text, **kwargs):
    """设置 x 轴标签"""
    return _rsplotlib.xlabel(text)


def ylabel(text, **kwargs):
    """设置 y 轴标签"""
    return _rsplotlib.ylabel(text)


def title(label, fontdict=None, **kwargs):
    """设置图表标题"""
    return _rsplotlib.title(label)


def grid(visible=True, **kwargs):
    """显示/隐藏网格"""
    c = kwargs.get('c')
    ls = kwargs.get('linestyle') or kwargs.get('ls')
    lw = kwargs.get('linewidth') or kwargs.get('lw')
    axis = kwargs.get('axis')
    return _rsplotlib.grid(visible, c, ls, lw, axis)


def legend(loc='best', **kwargs):
    """显示图例"""
    return _rsplotlib.legend(loc)


def xlim(left=None, right=None, **kwargs):
    """设置/获取 x 轴范围"""
    return _rsplotlib.xlim(left, right)


def ylim(bottom=None, top=None, **kwargs):
    """设置/获取 y 轴范围"""
    return _rsplotlib.ylim(bottom, top)


def xticks(ticks=None, labels=None, **kwargs):
    """设置 x 轴刻度"""
    ticks = _to_list(ticks)
    return _rsplotlib.xticks(ticks, labels)


def yticks(ticks=None, labels=None, **kwargs):
    """设置 y 轴刻度"""
    ticks = _to_list(ticks)
    return _rsplotlib.yticks(ticks, labels)


def xscale(scale, **kwargs):
    """设置 x 轴缩放"""
    return _rsplotlib.xscale(scale)


def yscale(scale, **kwargs):
    """设置 y 轴缩放"""
    return _rsplotlib.yscale(scale)


def margins(x_margin=None, y_margin=None, **kwargs):
    """设置自动缩放的边距"""
    return _rsplotlib.margins(x_margin, y_margin)


def box(on=None):
    """设置坐标轴边框"""
    return _rsplotlib.box_(on)


def minorticks_on():
    """显示次要刻度"""
    return _rsplotlib.minorticks_on()


def minorticks_off():
    """隐藏次要刻度"""
    return _rsplotlib.minorticks_off()


# ==================== 子图与布局 ====================

def subplots(nrows=1, ncols=1, figsize=None, dpi=None, **kwargs):
    """创建子图网格"""
    return _rsplotlib.subplots(nrows, ncols, figsize, dpi)


def subplot(nrows, ncols, index, **kwargs):
    """创建单个子图"""
    return _rsplotlib.subplot(nrows, ncols, index)


def tight_layout(**kwargs):
    """自动调整子图布局"""
    return _rsplotlib.tight_layout()


def subplots_adjust(left=None, right=None, bottom=None, top=None, wspace=None, hspace=None):
    """调整子图布局参数"""
    fig = _get_figure()
    if fig is not None:
        fig.subplots_adjust(left, right, bottom, top, wspace, hspace)


def set_size(width, height):
    """设置图形尺寸"""
    return _rsplotlib.set_size(width, height)


def twinx():
    """创建共享 x 轴的双 y 轴"""
    return _rsplotlib.twinx()


def twiny():
    """创建共享 y 轴的双 x 轴"""
    return _rsplotlib.twiny()


# ==================== 图形控制 ====================

def figure(num=None, figsize=None, dpi=None, **kwargs):
    """创建新图形

    Args:
        num: 图形编号 (未使用, 兼容 matplotlib)
        figsize: (width, height) 元组，单位为英寸
        dpi: 分辨率
    """
    fig = _rsplotlib.figure()
    d = dpi if dpi is not None else DEFAULT_DPI
    fig.set_dpi(d)
    if figsize is not None:
        w_inch, h_inch = figsize
        fig.set_size(round(w_inch * d), round(h_inch * d))
    else:
        w, h = rcParams.get('figure.figsize', list(DEFAULT_FIGSIZE))
        fig.set_size(round(w * d), round(h * d))
    return fig


def savefig(fname, **kwargs):
    """保存图形"""
    _rsplotlib.savefig(fname)


def show(**kwargs):
    """显示图形"""
    return _rsplotlib.show()


def gca(**kwargs):
    """获取当前 Axes"""
    return _rsplotlib.gca()


def gcf(**kwargs):
    """获取当前 Figure"""
    return _rsplotlib.gcf()


def cla():
    """清空当前 Axes"""
    return _rsplotlib.cla()


def clf():
    """清空当前 Figure"""
    return _rsplotlib.clf()


def close(fig=None):
    """关闭图形

    Args:
        fig: 图形或 'all' (兼容 matplotlib)
    """
    return _rsplotlib.close()


def axis(arg=None, **kwargs):
    """坐标轴控制

    支持:
        axis('off'): 隐藏坐标轴
        axis('equal'): 等比例
        axis('tight'): 紧凑
    """
    if arg == 'off':
        try:
            _rsplotlib.gca()._axis_off()
        except Exception:
            pass
    elif arg in ('equal', 'scaled'):
        gca().set_aspect('equal')
    return None


def colorbar(mappable=None, **kwargs):
    """添加颜色条 (占位)"""
    pass


# ==================== rcParams 重新导出 ====================
# rcParams / rcParamsOrig 从 _rcparams 模块导入，提供统一的配置访问
# （保留此注释以便读者了解 rcParams 的来源）


def get_cmap(name=None, lut=None):
    """获取颜色映射 (占位)"""
    return name
