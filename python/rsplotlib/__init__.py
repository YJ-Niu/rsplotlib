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

__version__ = "0.3.3"


def _round_float_for_display(value):
    if isinstance(value, float):
        rounded = round(value, 15)
        rounded_int = round(rounded)
        if abs(rounded - rounded_int) < 1e-10:
            return rounded_int
        return rounded
    return value


def _patch_rsnumpy_repr():
    try:
        import rsnumpy as np
        ndarray_cls = np.ndarray
        
        original_repr = ndarray_cls.__repr__
        original_str = ndarray_cls.__str__
        
        def patched_repr(self):
            try:
                data = self.tolist()
                
                def convert_to_python(obj):
                    if hasattr(obj, 'tolist'):
                        return convert_to_python(obj.tolist())
                    elif isinstance(obj, list):
                        return [convert_to_python(item) for item in obj]
                    elif isinstance(obj, complex):
                        real_part = _round_float_for_display(obj.real)
                        imag_part = _round_float_for_display(obj.imag)
                        return complex(real_part, imag_part)
                    else:
                        return _round_float_for_display(obj)
                
                converted_data = convert_to_python(data)
                
                if isinstance(converted_data, list) and len(converted_data) == 1:
                    converted_data = converted_data[0]
                
                def format_list(lst):
                    if not lst:
                        return "[]"
                    first = lst[0]
                    if isinstance(first, list):
                        inner = ", ".join(format_list(item) for item in lst)
                        return f"[{inner}]"
                    elif isinstance(first, complex):
                        formatted = [f"({x.real}+{x.imag}j)" for x in lst]
                        return f"[{', '.join(formatted)}]"
                    else:
                        return str(lst)
                
                if isinstance(converted_data, list):
                    return format_list(converted_data)
                else:
                    return str(converted_data)
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
