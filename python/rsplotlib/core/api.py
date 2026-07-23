"""rsplotlib API 完整函数定义

此模块包含 rsplotlib 库所有函数的 Python 包装，显示完整的参数签名和默认值。
"""

# 使用别名导入原始模块，避免与包装函数重名
from .. import rsplotlib as _rsplotlib
# 导入原始类
from ..rsplotlib import Figure, Axes
import warnings as _warnings


# ==================== 内部辅助函数 ====================

def _round_float(value):
    """对浮点数进行合理的四舍五入，避免精度问题"""
    if isinstance(value, float):
        rounded = round(value, 15)
        rounded_int = round(rounded)
        if abs(rounded - rounded_int) < 1e-10:
            return rounded_int
        return rounded
    return value


def _to_list(obj):
    """将数组对象或其他可迭代对象转换为 Python list"""
    if obj is None:
        return None
    if hasattr(obj, 'tolist'):
        result = obj.tolist()
        if isinstance(result, list):
            return [_round_float(item) for item in result]
        return _round_float(result)
    if isinstance(obj, (list, tuple)):
        return [_round_float(item) for item in obj]
    return _round_float(obj)


def _to_list_recursive(obj):
    """递归转换嵌套的数组对象为 Python list"""
    if obj is None:
        return None
    if hasattr(obj, 'tolist'):
        result = obj.tolist()
        if isinstance(result, list):
            return _to_list_recursive(result)
        return _round_float(result)
    if isinstance(obj, (list, tuple)):
        return [_to_list_recursive(item) for item in obj]
    return _round_float(obj)


# ==================== 绘图函数 ====================

def plot(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None,
         c=None, lw=None, ls=None, markersize=None, markeredgewidth=None,
         solid_capstyle=None):
    """绘制折线图

    Args:
        x: x 轴数据
        y: y 轴数据
        label: 图例标签 (默认: None)
        color: 颜色 (默认: None, 使用默认色循环)
        linestyle: 线型 (默认: None, 实线)
        marker: 标记样式 (默认: None)
        linewidth: 线宽 (默认: None)
        c: color 的 matplotlib 别名
        lw: linewidth 的 matplotlib 别名
        ls: linestyle 的 matplotlib 别名
        markersize: 标记大小
        markeredgewidth: 标记边缘宽度
        solid_capstyle: 端点形状 ('butt' | 'round' | 'projecting')
    """
    # 别名兜底：如果只传了 c/lw/ls 而主参数为 None，使用别名
    if color is None and c is not None:
        color = c
    if linewidth is None and lw is not None:
        linewidth = lw
    if linestyle is None and ls is not None:
        linestyle = ls
    return _rsplotlib.plot(
        _to_list(x), _to_list(y),
        label=label, color=color, linestyle=linestyle, marker=marker,
        linewidth=linewidth, lw=lw, c=c, ls=ls, markersize=markersize,
        markeredgewidth=markeredgewidth, solid_capstyle=solid_capstyle,
    )


def scatter(x, y, s=20.0, c=None, marker='o', label=None, alpha=1.0, color=None):
    """绘制散点图

    Args:
        x: x 轴数据
        y: y 轴数据
        s: 点大小 (默认: 20.0)
        c: 颜色 (默认: None)
        marker: 标记样式 (默认: 'o')
        label: 图例标签 (默认: None)
        alpha: 透明度 (默认: 1.0)
        color: 颜色别名 (默认: None)
    """
    # 支持 color 作为 c 的别名
    if c is None and color is not None:
        c = color
    return _rsplotlib.scatter(_to_list(x), _to_list(y), s, c, marker, label, alpha)


def bar(x, height, width=0.8, color=None, label=None):
    """绘制柱状图

    Args:
        x: x 轴位置
        height: 柱高度
        width: 柱宽度 (默认: 0.8)
        color: 颜色 (默认: None)
        label: 图例标签 (默认: None)
    """
    return _rsplotlib.bar(_to_list(x), _to_list(height), width, color, label)


def barh(y, width, height=0.8, color=None, label=None):
    """绘制水平柱状图

    Args:
        y: y 轴位置
        width: 柱宽度
        height: 柱高度 (默认: 0.8)
        color: 颜色 (默认: None)
        label: 图例标签 (默认: None)
    """
    return _rsplotlib.barh(_to_list(y), _to_list(width), height, color, label)


def hist(x, bins=10, range=None, density=False, weights=None,
         cumulative=False, bottom=None, histtype='bar', align='mid',
         orientation='vertical', rwidth=None, log=False, color=None,
         facecolor=None, label=None, stacked=False, alpha=1.0):
    """绘制直方图

    Args:
        x: 数据 (一维数组, 或多组数据组成的列表)
        bins: 分箱数量 (默认 10) 或箱边界列表
        range: 值域范围 (lo, hi)
        density: 是否归一化为概率密度 (默认 False)
        weights: 每个数据点的权重
        cumulative: 是否绘制累积分布 (True/False/-1)
        bottom: 每个柱子的起始基线
        histtype: 'bar' | 'barstacked' | 'step' | 'stepfilled'
        align: 'left' | 'mid' | 'right'
        orientation: 'vertical' | 'horizontal'
        rwidth: 柱子相对分箱宽度比例 (0~1)
        log: 计数轴是否对数刻度
        color / facecolor: 颜色或颜色列表
        label: 图例标签
        stacked: 是否堆叠
        alpha: 透明度 (默认 1.0)
    """
    if cumulative is True:
        cum = 1
    elif cumulative is False or cumulative is None:
        cum = 0
    else:
        cum = int(cumulative)
    weights_arg = _to_list_recursive(weights) if weights is not None else None
    range_arg = tuple(range) if range is not None else None
    return _rsplotlib.hist(
        _to_list_recursive(x), bins=bins, range=range_arg, density=density,
        weights=weights_arg, cumulative=cum, bottom=bottom, histtype=histtype,
        align=align, orientation=orientation, rwidth=rwidth, log=log,
        color=color, facecolor=facecolor, label=label, stacked=stacked,
        alpha=alpha,
    )


def pie(x, labels=None, colors=None, autopct=False):
    """绘制饼图

    Args:
        x: 数据
        labels: 标签列表 (默认: None)
        colors: 颜色列表 (默认: None)
        autopct: 是否显示百分比 (默认: False)
    """
    # 将 bool 类型的 autopct 转换为字符串格式
    if autopct is True:
        autopct_str = "%1.1f%%"
    elif isinstance(autopct, str):
        autopct_str = autopct
    else:
        autopct_str = None
    return _rsplotlib.pie(_to_list(x), labels, colors, autopct_str)


def boxplot(x, labels=None, vert=True):
    """绘制箱线图

    Args:
        x: 数据列表
        labels: 标签列表 (默认: None)
        vert: 是否垂直显示 (默认: True)
    """
    return _rsplotlib.boxplot(_to_list_recursive(x), labels, vert)


def fill_between(x, y1, y2=None, color=None, alpha=1.0, label=None):
    """填充区域

    Args:
        x: x 轴数据
        y1: 上边界
        y2: 下边界 (默认: None, 0)
        color: 颜色 (默认: None)
        alpha: 透明度 (默认: 1.0)
        label: 图例标签 (默认: None)
    """
    return _rsplotlib.fill_between(_to_list(x), _to_list(y1), _to_list(y2), color, alpha, label)


def errorbar(x, y, yerr=None, xerr=None, fmt='o', color=None, label=None, capsize=3.0):
    """绘制误差棒图

    Args:
        x: x 轴数据
        y: y 轴数据
        yerr: y 方向误差 (默认: None)
        xerr: x 方向误差 (默认: None)
        fmt: 标记格式 (默认: 'o')
        color: 颜色 (默认: None)
        label: 图例标签 (默认: None)
        capsize: 误差帽大小 (默认: 3.0)
    """
    return _rsplotlib.errorbar(_to_list(x), _to_list(y), _to_list(yerr), _to_list(xerr), fmt, color, label, capsize)


def stem(x, y, linefmt=None, markerfmt=None, label=None):
    """绘制茎叶图

    Args:
        x: x 轴数据
        y: y 轴数据
        linefmt: 茎线样式 (默认: None)
        markerfmt: 标记样式 (默认: None)
        label: 图例标签 (默认: None)
    """
    return _rsplotlib.stem(_to_list(x), _to_list(y), linefmt, markerfmt, label)


def step(x, y, where_='pre', label=None, color=None, linestyle='-', linewidth=1.5):
    """绘制阶梯图

    Args:
        x: x 轴数据
        y: y 轴数据
        where_: 阶梯位置 ('pre', 'post', 'mid', 默认: 'pre')
        label: 图例标签 (默认: None)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: '-')
        linewidth: 线宽 (默认: 1.5)
    """
    return _rsplotlib.step(_to_list(x), _to_list(y), where_, label, color, linestyle, linewidth)


def imshow(x, cmap='gray', aspect='auto', vmin=None, vmax=None,
           alpha=None, origin=None):
    """显示图像

    Args:
        x: 2D 标量数组 (经 cmap 上色) 或 3D RGB(A) 数组 (H, W, 3/4)
        cmap: 色图 (默认: 'gray', 可选: 'hot', 'cool')，仅对 2D 数据生效
        aspect: 纵横比 (默认: 'auto', 可选: 'equal')
        vmin, vmax: 2D 数据的颜色映射值域 (缺省取数据 min/max)
        alpha: 图像整体透明度 (0.0-1.0)
        origin: 'upper' (默认) 或 'lower'
    """
    return _rsplotlib.imshow(_to_list_recursive(x), cmap, aspect,
                             vmin, vmax, alpha, origin)


def violinplot(dataset, positions=None, widths=0.5, showmeans=False, showmedians=True):
    """绘制小提琴图

    Args:
        dataset: 数据集，可以是数组列表或 2D 数组
        positions: 位置数组 (默认: None)
        widths: 小提琴宽度 (默认: 0.5)
        showmeans: 是否显示均值 (默认: False)
        showmedians: 是否显示中位数 (默认: True)
    """
    try:
        return _rsplotlib.violinplot(dataset, positions, widths, showmeans, showmedians)
    except AttributeError:
        _warnings.warn("violinplot is not yet implemented in rsplotlib, using boxplot instead")
        return boxplot(dataset)


def hexbin(x, y, gridsize=100, cmap='hot', bins='log', mincnt=1):
    """绘制六边形分箱图

    Args:
        x: x 坐标数组
        y: y 坐标数组
        gridsize: 网格大小 (默认: 100)
        cmap: 色图 (默认: 'hot')
        bins: 分箱方式 (默认: 'log')
        mincnt: 最小计数 (默认: 1)
    """
    try:
        return _rsplotlib.hexbin(x, y, gridsize, cmap, bins, mincnt)
    except AttributeError:
        _warnings.warn("hexbin is not yet implemented in rsplotlib, using scatter instead")
        return scatter(x, y, s=10, alpha=0.5)


def contour(X, Y, Z, levels=None, colors=None, linestyles=None):
    """绘制等高线图

    Args:
        X: x 坐标网格
        Y: y 坐标网格
        Z: z 值数组
        levels: 等高线级别 (默认: None)
        colors: 颜色 (默认: None)
        linestyles: 线型 (默认: None)
    """
    try:
        return _rsplotlib.contour(X, Y, Z, levels, colors, linestyles)
    except AttributeError:
        _warnings.warn("contour is not yet implemented in rsplotlib")
        return None


def contourf(X, Y, Z, levels=None, cmap='coolwarm', alpha=1.0):
    """绘制填充等高线图

    Args:
        X: x 坐标网格
        Y: y 坐标网格
        Z: z 值数组
        levels: 等高线级别 (默认: None)
        cmap: 色图 (默认: 'coolwarm')
        alpha: 透明度 (默认: 1.0)
    """
    try:
        return _rsplotlib.contourf(X, Y, Z, levels, cmap, alpha)
    except AttributeError:
        _warnings.warn("contourf is not yet implemented in rsplotlib")
        return None


def stackplot(x, *args, labels=None, colors=None, alpha=1.0):
    """绘制堆叠面积图

    Args:
        x: x 轴数据
        *args: 多个 y 数据数组
        labels: 标签列表 (默认: None)
        colors: 颜色列表 (默认: None)
        alpha: 透明度 (默认: 1.0)
    """
    y_data = list(args) if args else []
    if y_data and len(y_data) == 1 and isinstance(y_data[0], (list, tuple)) and isinstance(y_data[0][0], (list, tuple)):
        y_data = list(y_data[0])
    return _rsplotlib.stackplot(_to_list(x), y_data, labels, colors, alpha)


# ==================== 辅助元素 ====================

def text(x, y, text, fontsize=None, color=None):
    """添加文本

    Args:
        x: x 位置
        y: y 位置
        text: 文本内容
        fontsize: 字体大小 (默认: None)
        color: 颜色 (默认: None)
    """
    return _rsplotlib.text(x, y, text, fontsize, color)


def axhline(y=None, color=None, linestyle=None, linewidth=None):
    """添加水平参考线

    Args:
        y: y 位置 (默认: None, 0)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.axhline(y, color, linestyle, linewidth)


def axvline(x=None, color=None, linestyle=None, linewidth=None):
    """添加垂直参考线

    Args:
        x: x 位置 (默认: None, 0)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.axvline(x, color, linestyle, linewidth)


def hlines(y, xmin, xmax, color=None, linestyle=None, linewidth=None):
    """绘制水平线段

    Args:
        y: y 位置
        xmin: 线段起点 x
        xmax: 线段终点 x
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.hlines(y, xmin, xmax, color, linestyle, linewidth)


def vlines(x, ymin, ymax, color=None, linestyle=None, linewidth=None):
    """绘制垂直线段

    Args:
        x: x 位置
        ymin: 线段起点 y
        ymax: 线段终点 y
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.vlines(x, ymin, ymax, color, linestyle, linewidth)


# ==================== 配置函数 ====================

def xlabel(text):
    """设置 x 轴标签

    Args:
        text: 标签文本
    """
    return _rsplotlib.xlabel(text)


def ylabel(text):
    """设置 y 轴标签

    Args:
        text: 标签文本
    """
    return _rsplotlib.ylabel(text)


def title(text):
    """设置图表标题

    Args:
        text: 标题文本
    """
    return _rsplotlib.title(text)


def grid(visible=True):
    """显示/隐藏网格

    Args:
        visible: 是否显示网格 (默认: True)
    """
    return _rsplotlib.grid(visible)


def legend(loc='best'):
    """显示图例

    Args:
        loc: 位置 (默认: 'best', 可选: 'upper right', 'upper left',
              'lower right', 'lower left', 'upper center')
    """
    return _rsplotlib.legend(loc)


def xlim(left, right):
    """设置 x 轴范围

    Args:
        left: 左边界
        right: 右边界
    """
    return _rsplotlib.xlim(left, right)


def ylim(bottom, top):
    """设置 y 轴范围

    Args:
        bottom: 下边界
        top: 上边界
    """
    return _rsplotlib.ylim(bottom, top)


def xticks(ticks=None, labels=None):
    """设置 x 轴刻度

    Args:
        ticks: 刻度位置列表 (默认: None)
        labels: 刻度标签列表 (默认: None)
    """
    return _rsplotlib.xticks(ticks, labels)


def yticks(ticks=None, labels=None):
    """设置 y 轴刻度

    Args:
        ticks: 刻度位置列表 (默认: None)
        labels: 刻度标签列表 (默认: None)
    """
    return _rsplotlib.yticks(ticks, labels)


# ==================== 子图与布局 ====================

def subplots(nrows=1, ncols=1, figsize=None, dpi=None, width_ratios=None, height_ratios=None, layout=None):
    """创建子图网格（兼容 Matplotlib 风格参数）

    Args:
        nrows: 行数 (默认: 1)
        ncols: 列数 (默认: 1)
        figsize: (width, height) 元组，单位为英寸
        dpi: 分辨率
        width_ratios: 列宽度比例
        height_ratios: 行高度比例
        layout: 布局模式，支持 'constrained'、'tight' 或 None

    Returns:
        tuple: (Figure, axes_list)
    """
    return _rsplotlib.subplots(nrows, ncols, figsize=figsize, dpi=dpi, width_ratios=width_ratios, height_ratios=height_ratios, layout=layout)


def subplot(nrows, ncols, index):
    """创建单个子图

    Args:
        nrows: 总行数
        ncols: 总列数
        index: 子图索引 (从1开始)

    Returns:
        Axes: 创建的子图
    """
    return _rsplotlib.subplot(nrows, ncols, index)


def tight_layout():
    """自动调整子图布局"""
    return _rsplotlib.tight_layout()


def set_size(width, height):
    """设置图形尺寸

    Args:
        width: 宽度 (像素)
        height: 高度 (像素)
    """
    return _rsplotlib.set_size(width, height)


def twinx():
    """创建共享 x 轴的双 y 轴

    Returns:
        Axes: 新的 y 轴
    """
    return _rsplotlib.twinx()


def twiny():
    """创建共享 y 轴的双 x 轴

    Returns:
        Axes: 新的 x 轴
    """
    return _rsplotlib.twiny()


# ==================== 图形控制 ====================

def figure(num=None, figsize=None, dpi=None, layout=None):
    """创建新图形（兼容 Matplotlib 风格的 `figsize`、`dpi` 和 `layout` 参数）

    Args:
        num: 图形编号（兼容，未使用）
        figsize: (width, height) 元组，单位为英寸
        dpi: 分辨率
        layout: 布局模式，支持 'constrained'、'tight' 或 None

    Returns:
        Figure: 创建的图形对象
    """
    return _rsplotlib.figure(figsize=figsize, dpi=dpi, layout=layout)


def savefig(filename):
    """保存图形

    Args:
        filename: 文件名 (支持 .svg 和 .png)
    """
    return _rsplotlib.savefig(filename)


def show():
    """显示图形 (保存到默认位置)"""
    return _rsplotlib.show()


def gca():
    """获取当前 Axes

    Returns:
        Axes: 当前坐标轴
    """
    return _rsplotlib.gca()


def gcf():
    """获取当前 Figure

    Returns:
        Figure: 当前图形对象
    """
    return _rsplotlib.gcf()


def cla():
    """清空当前 Axes"""
    return _rsplotlib.cla()


def clf():
    """清空当前 Figure"""
    return _rsplotlib.clf()


def close():
    """关闭当前 Figure"""
    return _rsplotlib.close()


# ==================== 对数坐标 ====================

def semilogx(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None):
    """x 轴对数坐标折线图

    Args:
        x: x 轴数据
        y: y 轴数据
        label: 图例标签 (默认: None)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        marker: 标记样式 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.semilogx(_to_list(x), _to_list(y), label, color, linestyle, marker, linewidth)


def semilogy(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None):
    """y 轴对数坐标折线图

    Args:
        x: x 轴数据
        y: y 轴数据
        label: 图例标签 (默认: None)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        marker: 标记样式 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.semilogy(_to_list(x), _to_list(y), label, color, linestyle, marker, linewidth)


def loglog(x, y, label=None, color=None, linestyle=None, marker=None, linewidth=None):
    """双对数坐标折线图

    Args:
        x: x 轴数据
        y: y 轴数据
        label: 图例标签 (默认: None)
        color: 颜色 (默认: None)
        linestyle: 线型 (默认: None)
        marker: 标记样式 (默认: None)
        linewidth: 线宽 (默认: None)
    """
    return _rsplotlib.loglog(_to_list(x), _to_list(y), label, color, linestyle, marker, linewidth)


# ==================== 样式控制 ====================

def use(backend):
    """选择后端 (兼容 matplotlib API)

    Args:
        backend: 后端名称 (如 'Agg', 'SVG')
    """
    _rsplotlib.use_(backend)


def xscale(scale):
    """设置 x 轴缩放
    Args:
        scale: 缩放类型 ('linear', 'log', 'symlog', 'logit')
    """
    return _rsplotlib.xscale(scale)


def yscale(scale):
    """设置 y 轴缩放
    Args:
        scale: 缩放类型 ('linear', 'log', 'symlog', 'logit')
    """
    return _rsplotlib.yscale(scale)


def margins(x_margin=None, y_margin=None):
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


# ==================== 模块导出 ====================

__all__ = [
    # 绘图函数
    'plot', 'scatter', 'bar', 'barh', 'hist', 'pie', 'boxplot',
    'fill_between', 'errorbar', 'stem', 'step', 'imshow',
    # 辅助元素
    'text', 'axhline', 'axvline', 'hlines', 'vlines',
    # 配置函数
    'xlabel', 'ylabel', 'title', 'grid', 'legend',
    'xlim', 'ylim', 'xticks', 'yticks',
    'xscale', 'yscale', 'margins', 'box', 'minorticks_on', 'minorticks_off',
    # 子图与布局
    'subplots', 'subplot', 'tight_layout', 'set_size', 'twinx', 'twiny',
    # 图形控制
    'figure', 'savefig', 'show', 'gca', 'cla', 'clf', 'close', 'gcf',
    # 对数坐标
    'semilogx', 'semilogy', 'loglog',
    # 样式
    'use',
    # 类
    'Figure', 'Axes',
]
