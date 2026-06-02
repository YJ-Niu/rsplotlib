"""rsplot.ticker - Matplotlib ticker 兼容接口

提供刻度定位器和格式化器类。
"""

import math


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
    
    Args:
        base: 刻度间距
    """

    def __init__(self, base=1.0):
        self.base = base

    def tick_values(self, vmin, vmax):
        if self.base == 0:
            return []
        vmin = math.floor(vmin / self.base) * self.base
        vmax = math.ceil(vmax / self.base) * self.base
        ticks = []
        v = vmin
        while v <= vmax + self.base * 0.5:
            ticks.append(v)
            v += self.base
        return ticks

    def __repr__(self):
        return f'MultipleLocator(base={self.base})'


class MaxNLocator(Locator):
    """最大数量定位器 - 最多 nbins+1 个刻度
    
    Args:
        nbins: 最大区间数 (默认: 10)
        integer: 是否只使用整数 (默认: False)
    """

    def __init__(self, nbins=10, integer=False):
        self.nbins = nbins
        self.integer = integer

    def tick_values(self, vmin, vmax):
        if vmax <= vmin:
            return [vmin]
        range_val = vmax - vmin
        if range_val == 0:
            return [vmin]
        raw_step = range_val / self.nbins
        if self.integer:
            step = max(1, int(raw_step))
        else:
            step = self._nice_step(raw_step)
        vmin = math.floor(vmin / step) * step
        ticks = []
        v = vmin
        while v <= vmax + step * 0.5:
            if not self.integer or abs(v - round(v)) < 1e-10:
                if self.integer:
                    v = round(v)
                if not ticks or abs(v - ticks[-1]) > 1e-10:
                    ticks.append(v)
            v += step
        while len(ticks) > self.nbins + 1:
            ticks = ticks[::2]
        return ticks

    def _nice_step(self, step):
        if step <= 0:
            return 1.0
        exponent = math.floor(math.log10(step))
        fraction = step / (10 ** exponent)
        if fraction < 1.5:
            nice = 1.0
        elif fraction < 3.5:
            nice = 2.0
        elif fraction < 7.5:
            nice = 5.0
        else:
            nice = 10.0
        return nice * (10 ** exponent)

    def __repr__(self):
        return f'MaxNLocator(nbins={self.nbins}, integer={self.integer})'


class AutoLocator(MaxNLocator):
    """自动定位器"""

    def __init__(self):
        super().__init__(nbins=10)


class AutoMinorLocator(Locator):
    """自动次要刻度定位器
    
    Args:
        n: 每个主要间隔中的次要刻度数 (默认: 5)
    """

    def __init__(self, n=5):
        self.n = n

    def tick_values(self, vmin, vmax):
        major_step = (vmax - vmin) / 10
        if major_step <= 0:
            return []
        minor_step = major_step / self.n
        ticks = []
        v = vmin
        while v <= vmax + minor_step * 0.5:
            ticks.append(v)
            v += minor_step
        return ticks

    def __repr__(self):
        return f'AutoMinorLocator(n={self.n})'


class FixedLocator(Locator):
    """固定位置定位器"""

    def __init__(self, locs):
        self.locs = list(locs)

    def tick_values(self, vmin, vmax):
        return [l for l in self.locs if vmin <= l <= vmax]


class LinearLocator(Locator):
    """线性定位器"""

    def __init__(self, numticks=10):
        self.numticks = numticks

    def tick_values(self, vmin, vmax):
        if vmax <= vmin:
            return [vmin]
        step = (vmax - vmin) / (self.numticks - 1)
        return [vmin + i * step for i in range(self.numticks)]


class LogLocator(Locator):
    """对数定位器"""

    def __init__(self, base=10.0, numticks=10):
        self.base = base
        self.numticks = numticks

    def tick_values(self, vmin, vmax):
        if vmin <= 0:
            vmin = 1e-10
        log_min = math.log(vmin, self.base)
        log_max = math.log(vmax, self.base)
        step = (log_max - log_min) / (self.numticks - 1)
        return [self.base ** (log_min + i * step) for i in range(self.numticks)]


class NullLocator(Locator):
    """空定位器 - 不显示刻度"""

    def tick_values(self, vmin, vmax):
        return []


# ==================== 格式化器 ====================

class Formatter:
    """刻度格式化器基类"""

    def format_ticks(self, values):
        return [self(val) for val in values]

    def __call__(self, value):
        return str(value)


class NullFormatter(Formatter):
    """不显示标签"""

    def __call__(self, value):
        return ''


class FixedFormatter(Formatter):
    """固定标签格式化器"""

    def __init__(self, seq):
        self.seq = list(seq)

    def __call__(self, value):
        try:
            idx = int(round(value))
            if 0 <= idx < len(self.seq):
                return str(self.seq[idx])
        except (ValueError, IndexError):
            pass
        return ''


class FormatStrFormatter(Formatter):
    """格式化字符串"""

    def __init__(self, fmt):
        self.fmt = fmt

    def __call__(self, value):
        return self.fmt % value


class ScalarFormatter(Formatter):
    """标量格式化器"""

    def __call__(self, value):
        if abs(value) >= 1e4 or abs(value) < 1e-3:
            return f'{value:.2e}'
        return f'{value:g}'


class LogFormatterSciNotation(Formatter):
    """科学计数法格式化器"""

    def __call__(self, value):
        if value <= 0:
            return '0'
        exp = math.log10(value)
        return f'$10^{{{int(exp)}}}$'


class FuncFormatter(Formatter):
    """函数格式化器"""

    def __init__(self, func):
        self.func = func

    def __call__(self, value):
        return self.func(value)


class StrMethodFormatter(Formatter):
    """字符串方法格式化器"""

    def __init__(self, fmt):
        self.fmt = fmt

    def __call__(self, value):
        return self.fmt.format(value)


# ==================== 便捷函数 ====================

def AutoLocator():
    """创建自动定位器"""
    return MaxNLocator(nbins=10, integer=False)