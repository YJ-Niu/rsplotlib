"""rsplotlib._rcparams - Matplotlib 兼容的 rcParams 配置管理

提供类似 matplotlib.rcParams 的全局配置字典，保存绘图相关的默认参数。
"""

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

    区别于普通 dict：当访问不存在的键时返回 None 而不是抛出 KeyError，
    这样可以安全地在代码中使用 `rcParams.get(key, default)` 而无需 try/except。
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.update(_DEFAULT_RC)

    def __getitem__(self, key):
        try:
            return super().__getitem__(key)
        except KeyError:
            return None

    def __setitem__(self, key, value):
        super().__setitem__(key, value)


# 全局单例
rcParams = RcParams()
