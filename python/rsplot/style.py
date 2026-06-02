"""rsplot.style - Matplotlib style 兼容接口"""

from . import rsplot as _rsplot


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
    """样式管理器"""

    def __init__(self):
        self._current_style = 'default'

    def use(self, style_name):
        """应用样式
        
        Args:
            style_name: 样式名称
        """
        self._current_style = style_name

    def available(self):
        """返回可用样式列表"""
        return AVAILABLE_STYLES

    @property
    def current(self):
        return self._current_style


# 全局样式实例
style = Style()


def use(style_name):
    """应用样式（模块级函数）"""
    style.use(style_name)


def available():
    """返回可用样式列表"""
    return style.available()