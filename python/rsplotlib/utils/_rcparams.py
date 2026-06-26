"""rsplotlib._rcparams - Matplotlib 兼容的 rcParams 配置管理

提供类似 matplotlib.rcParams 的全局配置字典，保存绘图相关的默认参数。

**字体钩子**：当用户设置 `rcParams["font.sans-serif"]` 时，会自动调用
Rust 的字体解析器 `apply_rcparams_font()`，把对应的字体文件注册到 plotters 的
字体数据库中，从而真正影响文本渲染（而不是像普通 dict 一样只更新值）。
"""
import copy as _copy
from typing import Any

from .. import rsplotlib as _rs

# 默认配置：与 matplotlib 保持一致的常用项
_DEFAULT_RC = {
    'font.sans-serif': ['Helvetica', 'Arial', 'sans-serif'],
    'axes.unicode_minus': True,
    'font.size': 10,
    'figure.figsize': [6.4, 4.8],
    'figure.dpi': 100.0,
}


class RcParams(dict):
    """与 matplotlib.rcParams 兼容的配置字典

    区别于普通 dict：
    1. 当访问不存在的键时返回 None 而不是抛出 KeyError。
    2. **设置 `font.sans-serif` 时自动调用 Rust 字体解析器**，把对应字体文件
       注册到 plotters 的字体数据库中，使 plotters 真正使用用户指定的字体
       渲染文字（而不是只更新一个不会生效的字符串列表）。
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.update(_DEFAULT_RC)

    def __getitem__(self, key):
        try:
            return super().__getitem__(key)
        except KeyError:
            return None

    def __setitem__(self, key: str, value: Any) -> None:
        super().__setitem__(key, value)
        # 字体钩子：用户在 Python 端改字体时同步通知 Rust 端注册
        if key == 'font.sans-serif':
            try:
                _rs.apply_rcparams_font()
            except Exception:
                # 注册失败不影响 rcParams 写入
                pass


# 全局单例
rcParams = RcParams()
# 原始默认配置副本（用于 rcParams.reset() 等恢复操作）
rcParamsOrig = _copy.deepcopy(rcParams)
