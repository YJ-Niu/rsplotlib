"""rsplotlib.ticker - 向后兼容的导入模块

此模块提供从旧路径 `rsplotlib.ticker` 导入的支持。
实际实现在 `rsplotlib.ticks.ticker` 中。
"""
from .ticks.ticker import *  # noqa: F403, F401