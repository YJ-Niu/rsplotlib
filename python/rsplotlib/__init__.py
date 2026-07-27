"""rsplotlib 包顶层导出。建议通过导入自 `rsplotlib.api` 使用公开 API。"""

from .core.api import *  # noqa: F403, F401
from .core.api import __all__ as _api_all
from .rsplotlib import register_sans_serif_font
from . import pyplot, pylab  # noqa: F401
from .utils import _font_resolver, style  # noqa: F401
from .layout import gridspec  # noqa: F401
from .ticks import ticker  # noqa: F401
import rsplotlib.text as text

GridSpec = gridspec.GridSpec
MaxNLocator = ticker.MaxNLocator
MultipleLocator = ticker.MultipleLocator
AutoMinorLocator = ticker.AutoMinorLocator

__version__ = "0.3.6"


def _round_float_for_display(value):
    if isinstance(value, float):
        rounded = round(value, 15)
        rounded_int = round(rounded)
        if abs(rounded - rounded_int) < 1e-10:
            return rounded_int
        return rounded
    return value


def _patch_rsnumpy_repr():
    """优化 rsnumpy.ndarray 的显示格式，使其输出更简洁易读。"""
    try:
        import rsnumpy as np
        ndarray_cls = np.ndarray
        
        original_repr = ndarray_cls.__repr__
        original_str = ndarray_cls.__str__
        
        def _convert_value(obj):
            """递归转换并四舍五入值。"""
            if hasattr(obj, 'tolist'):
                return _convert_value(obj.tolist())
            if isinstance(obj, list):
                return [_convert_value(item) for item in obj]
            if isinstance(obj, complex):
                return complex(
                    _round_float_for_display(obj.real),
                    _round_float_for_display(obj.imag),
                )
            return _round_float_for_display(obj)
        
        def _format_list(lst):
            """格式化嵌套列表为紧凑字符串表示。"""
            if not lst:
                return "[]"
            first = lst[0]
            if isinstance(first, list):
                inner = ", ".join(_format_list(item) for item in lst)
                return f"[{inner}]"
            if isinstance(first, complex):
                parts = [f"({x.real}+{x.imag}j)" for x in lst]
                return f"[{', '.join(parts)}]"
            return str(lst)
        
        def patched_repr(self):
            try:
                data = _convert_value(self.tolist())
                if isinstance(data, list) and len(data) == 1:
                    data = data[0]
                return _format_list(data) if isinstance(data, list) else str(data)
            except Exception:
                ndarray_cls.__str__ = original_str
                result = original_repr(self)
                ndarray_cls.__str__ = patched_str
                return result
        
        def patched_str(self):
            try:
                return patched_repr(self)
            except Exception:
                return original_str(self)
        
        ndarray_cls.__repr__ = patched_repr
        ndarray_cls.__str__ = patched_str
    except ImportError:
        pass


_patch_rsnumpy_repr()


__all__ = list(_api_all) + [
    'pyplot', 'style', 'gridspec', 'ticker', 'text',
    'GridSpec', 'MaxNLocator', 'MultipleLocator',
    'AutoMinorLocator', 'register_sans_serif_font',
]
