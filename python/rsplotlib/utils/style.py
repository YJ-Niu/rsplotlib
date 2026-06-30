"""rsplotlib.style - Matplotlib style 兼容接口

底层实现: Rust Style
"""

from .. import rsplotlib as _rs


# ==================== 可用样式 ====================

AVAILABLE_STYLES = [
    'default',
    'classic',
    'ggplot',
    'seaborn-v0_8',
    'fast',
    'fivethirtyeight',
    'grayscale',
    'dark_background',
    'bmh',
    'tableau-colorblind10',
]


class Style:
    """样式管理器

    底层实现: Rust Style
    """

    def __init__(self):
        self._impl = _rs.Style()

    def use(self, style_name):
        """应用样式

        Args:
            style_name: 样式名称
        """
        self._impl.use_(style_name)

    def available(self):
        """返回可用样式列表"""
        return self._impl.available()

    @property
    def current(self):
        return self._impl.current


# 全局样式实例
style = Style()


def use(style_name):
    """应用样式（模块级函数）"""
    style.use(style_name)


def available():
    """返回可用样式列表"""
    return style.available()


def current():
    """返回当前样式名称"""
    return style.current
