"""rsplotlib 包顶层导出。建议通过导入自 `rsplotlib.api` 使用公开 API。"""

from .api import *
from .api import __all__ as _api_all

from . import pyplot, style, gridspec, ticker

GridSpec = gridspec.GridSpec
MaxNLocator = ticker.MaxNLocator
MultipleLocator = ticker.MultipleLocator
AutoMinorLocator = ticker.AutoMinorLocator

__all__ = list(_api_all) + [
	'pyplot', 'style', 'gridspec', 'ticker',
	'GridSpec', 'MaxNLocator', 'MultipleLocator', 'AutoMinorLocator',
]