"""rsplotlib 包顶层导出。建议通过导入自 `rsplotlib.api` 使用公开 API。"""

from .core.api import *  # noqa: F403, F401
from .core.api import __all__ as _api_all
from .rsplotlib import register_sans_serif_font
from . import pyplot, pylab  # noqa: F401
from .utils import _font_resolver, style  # noqa: F401
from .layout import gridspec  # noqa: F401
from .ticks import ticker  # noqa: F401

GridSpec = gridspec.GridSpec
MaxNLocator = ticker.MaxNLocator
MultipleLocator = ticker.MultipleLocator
AutoMinorLocator = ticker.AutoMinorLocator

__version__ = "0.2.7"
# 从内部 Rust 模块导出字体注册函数

__all__ = list(_api_all) + [
    'pyplot', 'style', 'gridspec', 'ticker',
    'GridSpec', 'MaxNLocator', 'MultipleLocator',
    'AutoMinorLocator', 'register_sans_serif_font',
]
