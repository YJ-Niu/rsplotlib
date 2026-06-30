"""rsplotlib.gridspec - 向后兼容的导入模块

此模块提供从旧路径 `rsplotlib.gridspec` 导入的支持。
实际实现在 `rsplotlib.layout.gridspec` 中。
"""
from .layout.gridspec import *  # noqa: F403, F401
