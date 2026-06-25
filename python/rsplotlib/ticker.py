"""rsplotlib.ticker - Matplotlib ticker 兼容接口

提供刻度定位器和格式化器类。
底层实现已迁移至 Rust 层，此模块为保持完整 Python 接口的薄包装层。
"""

from . import rsplotlib as _rs


class Tick:
    """刻度对象"""

    def __init__(self, loc, label=''):
        self.loc = loc
        self.label = label


# ==================== 定位器 ====================

class Locator:
    """刻度定位器基类"""

    def tick_values(self, vmin, vmax):
        raise NotImplementedError

    def __call__(self):
        return []


class MultipleLocator(Locator):
    """倍数定位器 - 刻度位置是基数的整数倍

    底层实现: Rust MultipleLocator

    Args:
        base: 刻度间距
    """

    def __init__(self, base=1.0):
        self.base = base
        self._impl = _rs.MultipleLocator(base)

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)

    def __repr__(self):
        return f'MultipleLocator(base={self.base})'


class MaxNLocator(Locator):
    """最大数量定位器 - 最多 nbins+1 个刻度

    底层实现: Rust MaxNLocator

    Args:
        nbins: 最大区间数 (默认: 10)
        integer: 是否只使用整数 (默认: False)
    """

    def __init__(self, nbins=10, integer=False):
        self.nbins = nbins
        self.integer = integer
        self._impl = _rs.MaxNLocator(nbins, integer)

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)

    def __repr__(self):
        return f'MaxNLocator(nbins={self.nbins}, integer={self.integer})'


class AutoLocator(MaxNLocator):
    """自动定位器

    底层实现: Rust MaxNLocator(nbins=10)
    """

    def __init__(self):
        super().__init__(nbins=10)


class AutoMinorLocator(Locator):
    """自动次要刻度定位器

    底层实现: Rust AutoMinorLocator

    Args:
        n: 每个主要间隔中的次要刻度数 (默认: 5)
    """

    def __init__(self, n=5):
        self.n = n
        self._impl = _rs.AutoMinorLocator(n)

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)

    def __repr__(self):
        return f'AutoMinorLocator(n={self.n})'


class FixedLocator(Locator):
    """固定位置定位器

    底层实现: Rust FixedLocator
    """

    def __init__(self, locs):
        self.locs = list(locs)
        self._impl = _rs.FixedLocator(list(locs))

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)


class LinearLocator(Locator):
    """线性定位器

    底层实现: Rust LinearLocator
    """

    def __init__(self, numticks=10):
        self.numticks = numticks
        self._impl = _rs.LinearLocator(numticks)

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)


class LogLocator(Locator):
    """对数定位器

    底层实现: Rust LogLocator
    """

    def __init__(self, base=10.0, numticks=10):
        self.base = base
        self.numticks = numticks
        self._impl = _rs.LogLocator(base, numticks)

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)


class NullLocator(Locator):
    """空定位器 - 不显示刻度

    底层实现: Rust NullLocator
    """

    def __init__(self):
        self._impl = _rs.NullLocator()

    def tick_values(self, vmin, vmax):
        return self._impl.tick_values(vmin, vmax)


# ==================== 格式化器 ====================

class Formatter:
    """刻度格式化器基类"""

    def format_ticks(self, values):
        return [self(val) for val in values]

    def __call__(self, value):
        return str(value)


class NullFormatter(Formatter):
    """不显示标签

    底层实现: Rust NullFormatter
    """

    def __init__(self):
        self._impl = _rs.NullFormatter()

    def __call__(self, value):
        return self._impl.__call__(value)


class FixedFormatter(Formatter):
    """固定标签格式化器

    底层实现: Rust FixedFormatter
    """

    def __init__(self, seq):
        self.seq = list(seq)
        self._impl = _rs.FixedFormatter([str(s) for s in seq])

    def __call__(self, value):
        return self._impl.__call__(value)


class FormatStrFormatter(Formatter):
    """格式化字符串

    底层实现: Rust FormatStrFormatter
    """

    def __init__(self, fmt):
        self.fmt = fmt
        self._impl = _rs.FormatStrFormatter(fmt)

    def __call__(self, value):
        return self._impl.__call__(value)


class ScalarFormatter(Formatter):
    """标量格式化器

    底层实现: Rust ScalarFormatter
    """

    def __init__(self):
        self._impl = _rs.ScalarFormatter()

    def __call__(self, value):
        return self._impl.__call__(value)


class LogFormatterSciNotation(Formatter):
    """科学计数法格式化器

    底层实现: Rust LogFormatterSciNotation
    """

    def __init__(self):
        self._impl = _rs.LogFormatterSciNotation()

    def __call__(self, value):
        return self._impl.__call__(value)


class FuncFormatter(Formatter):
    """函数格式化器

    底层实现: Rust FuncFormatter
    """

    def __init__(self, func):
        self.func = func
        self._impl = _rs.FuncFormatter(func)

    def __call__(self, value):
        return self._impl.__call__(value)


class StrMethodFormatter(Formatter):
    """字符串方法格式化器

    底层实现: Rust StrMethodFormatter
    """

    def __init__(self, fmt):
        self.fmt = fmt
        self._impl = _rs.StrMethodFormatter(fmt)

    def __call__(self, value):
        return self._impl.__call__(value)


# ==================== 便捷函数 ====================

def auto_locator():
    """创建自动定位器"""
    return MaxNLocator(nbins=10, integer=False)