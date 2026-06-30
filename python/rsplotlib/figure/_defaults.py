"""rsplotlib._figure_defaults - 图形创建相关的默认值

集中管理 figure 尺寸、DPI 等默认值，避免散落在 pyplot 模块中。
底层实现: Rust figure 模块
"""

from .. import rsplotlib as _rs

# 默认图形尺寸（英寸），与 matplotlib 默认一致
DEFAULT_FIGSIZE = _rs.get_default_figsize()

# 默认 DPI
DEFAULT_DPI = _rs.get_default_dpi()
