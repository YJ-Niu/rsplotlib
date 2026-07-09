"""rsplotlib.dates - Matplotlib dates 兼容接口 (子集)。

提供日期刻度格式化器 ConciseDateFormatter，兼容 matplotlib.dates 的常用 API。
日期数值沿用 matplotlib 约定：浮点数表示自 1970-01-01 (UTC) 起的天数。
"""

import datetime as _datetime

from .ticks.ticker import Formatter

# matplotlib 自 3.3 起默认纪元为 1970-01-01
_EPOCH = _datetime.datetime(1970, 1, 1)


def num2date(x):
    """将 matplotlib 日期数值 (自纪元起的天数) 转为 datetime。"""
    return _EPOCH + _datetime.timedelta(days=float(x))


class ConciseDateFormatter(Formatter):
    """紧凑日期格式化器，兼容 matplotlib.dates.ConciseDateFormatter。

    根据刻度跨度自动选择合适的日期/时间粒度，尽量少地重复上层信息
    (年/月/日)，产出简洁的刻度标签。

    Args:
        locator: 关联的刻度定位器 (与 matplotlib 一致，可选用于确定跨度)。
        tz, formats, offset_formats, zero_formats, show_offset, usetex:
            为兼容 matplotlib 签名而接受，当前实现下部分参数不生效。
    """

    def __init__(self, locator=None, tz=None, formats=None,
                 offset_formats=None, zero_formats=None, show_offset=True,
                 *, usetex=None):
        self._locator = locator
        self._tz = tz
        self.formats = formats
        self.zero_formats = zero_formats
        self.offset_formats = offset_formats
        self.show_offset = show_offset
        self._usetex = usetex
        self.offset_string = ''

    def _to_datetime(self, value):
        if isinstance(value, _datetime.datetime):
            return value
        if isinstance(value, _datetime.date):
            return _datetime.datetime(value.year, value.month, value.day)
        if isinstance(value, str):
            return None
        return num2date(value)

    def _choose_scale(self, values):
        """根据刻度跨度选择格式粒度。返回 strftime 模式。"""
        dts = [self._to_datetime(v) for v in values]
        dts = [d for d in dts if d is not None]
        if len(dts) < 2:
            return '%b %d'
        span = max(dts) - min(dts)
        secs = abs(span.total_seconds())
        if secs > 365 * 24 * 3600:
            return '%Y'
        if secs > 30 * 24 * 3600:
            return '%Y-%m'
        if secs > 24 * 3600:
            return '%b %d'
        if secs > 3600:
            return '%H:%M'
        return '%H:%M:%S'

    def format_ticks(self, values):
        fmt = self._choose_scale(values)
        return [self._format_one(v, fmt) for v in values]

    def _format_one(self, value, fmt):
        dt = self._to_datetime(value)
        if dt is None:
            return str(value)
        return dt.strftime(fmt)

    def __call__(self, value, pos=None):
        return self._format_one(value, '%b %d')

    def __repr__(self):
        return 'ConciseDateFormatter()'
