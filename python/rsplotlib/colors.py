"""rsplotlib.colors - Matplotlib colors 兼容接口 (子集)。

提供归一化类 Normalize / LogNorm，用于将数据映射到 [0, 1] 供 colormap 上色。
当前 imshow/pcolormesh 等接受 norm 参数，Normalize/LogNorm 在此处提供兼容 API。
"""

import math


class Normalize:
    """将数据线性归一化到 [0, 1]，兼容 matplotlib.colors.Normalize。

    Args:
        vmin, vmax: 归一化值域端点，缺省时在首次调用时按数据自动推断。
        clip: 是否将越界值裁剪到 [0, 1]。
    """

    # 归一化类型标记：供 pyplot 把归一化方式（线性/对数）下沉给 Rust 上色与颜色条。
    _norm_kind = 'linear'

    def __init__(self, vmin=None, vmax=None, clip=False):
        self.vmin = vmin
        self.vmax = vmax
        self.clip = clip

    def autoscale_None(self, values):
        """当 vmin/vmax 未设置时，根据数据填充。"""
        seq = _flatten(values)
        if seq:
            if self.vmin is None:
                self.vmin = min(seq)
            if self.vmax is None:
                self.vmax = max(seq)

    def _normalize_scalar(self, value):
        vmin = 0.0 if self.vmin is None else float(self.vmin)
        vmax = 1.0 if self.vmax is None else float(self.vmax)
        if vmax == vmin:
            return 0.0
        t = (float(value) - vmin) / (vmax - vmin)
        if self.clip:
            t = max(0.0, min(1.0, t))
        return t

    def __call__(self, value, clip=None):
        if clip is not None:
            saved = self.clip
            self.clip = clip
            try:
                return self._apply(value)
            finally:
                self.clip = saved
        return self._apply(value)

    def _apply(self, value):
        if _is_sequence(value):
            return [self._normalize_scalar(v) for v in _flatten(value)]
        return self._normalize_scalar(value)

    def __repr__(self):
        return f'Normalize(vmin={self.vmin}, vmax={self.vmax})'


class LogNorm(Normalize):
    """对数归一化，兼容 matplotlib.colors.LogNorm。

    将数据按对数刻度映射到 [0, 1]；非正值被视为越界。
    """

    _norm_kind = 'log'

    def autoscale_None(self, values):
        """当 vmin/vmax 未设置时，用数据的最小正值 / 最大值填充（对数刻度要求正值）。"""
        seq = [v for v in _flatten(values) if v > 0]
        if seq:
            if self.vmin is None:
                self.vmin = min(seq)
            if self.vmax is None:
                self.vmax = max(seq)

    def _normalize_scalar(self, value):
        vmin = self.vmin
        vmax = self.vmax
        if vmin is None or vmax is None or vmin <= 0 or vmax <= 0:
            return 0.0
        v = float(value)
        if v <= 0:
            return 0.0
        t = (math.log(v) - math.log(vmin)) / (math.log(vmax) - math.log(vmin))
        if self.clip:
            t = max(0.0, min(1.0, t))
        return t

    def __repr__(self):
        return f'LogNorm(vmin={self.vmin}, vmax={self.vmax})'


def _is_sequence(obj):
    if obj is None or isinstance(obj, str):
        return False
    return hasattr(obj, 'tolist') or hasattr(obj, '__iter__')


def _flatten(values):
    """递归展开嵌套序列 / rsnumpy 数组为一维浮点列表。"""
    if values is None:
        return []
    if hasattr(values, 'tolist'):
        values = values.tolist()
    out = []
    if isinstance(values, (list, tuple)):
        for item in values:
            if isinstance(item, (list, tuple)):
                out.extend(_flatten(item))
            else:
                out.append(float(item))
    else:
        out.append(float(values))
    return out
