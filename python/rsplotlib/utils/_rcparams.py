"""rsplotlib._rcparams - Matplotlib 兼容的 rcParams 配置管理

提供类似 matplotlib.rcParams 的全局配置字典，保存绘图相关的默认参数。

**字体钩子**：当用户设置 `rcParams["font.sans-serif"]` 时，会自动调用
Rust 的字体解析器 `apply_rcparams_font()`，把对应的字体文件注册到 plotters 的
字体数据库中，从而真正影响文本渲染（而不是像普通 dict 一样只更新值）。
"""
import copy as _copy
from typing import Any

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
    2. **设置 `font.sans-serif` 时自动调用字体解析器**，把对应字体文件
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
        self._trigger_font_hook(key)

    def update(self, *args, **kwargs):
        super().update(*args, **kwargs)
        # update 也需要触发字体钩子，否则初始化时默认字体不会被注册
        if args:
            other = args[0]
            if isinstance(other, dict):
                keys = other.keys()
            else:
                keys = [k for k, _ in other]
        else:
            keys = kwargs.keys()
        if 'font.sans-serif' in keys:
            self._trigger_font_hook('font.sans-serif')

    def _trigger_font_hook(self, key: str) -> None:
        # 字体钩子：用户在 Python 端改字体时同步通知字体注册
        if key == 'font.sans-serif':
            try:
                # 延迟导入避免循环依赖
                from .. import rsplotlib as _rs
                from ._font_resolver import resolve_font_path
                sans_serif = self.get('font.sans-serif')
                if not sans_serif:
                    return
                if isinstance(sans_serif, str):
                    candidates = [sans_serif]
                else:
                    candidates = list(sans_serif)
                candidates = [c for c in candidates if c and c.lower() != 'sans-serif']
                # 清空旧的字体栈
                _rs.clear_font_stack()
                # 注册所有字体
                for family in candidates:
                    path = resolve_font_path(family)
                    if path is None:
                        import os
                        if os.path.isfile(family):
                            path = family
                    if path is not None:
                        try:
                            _rs.register_sans_serif_font(path, family)
                        except Exception:
                            pass
            except Exception:
                # 注册失败不影响 rcParams 写入
                pass


# 全局单例
rcParams = RcParams()
# 原始默认配置副本（用于 rcParams.reset() 等恢复操作）
rcParamsOrig = _copy.deepcopy(rcParams)
