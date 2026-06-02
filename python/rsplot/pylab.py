"""rsplot.pylab - Matplotlib pylab 兼容接口"""

from .pyplot import rcParams, rcParamsOrig, figure, gca, gcf

# mpl 模块可以从 pylab 导入
class MplModule:
    """兼容 matplotlib 的 mpl 模块"""

    def __init__(self):
        self.rcParams = rcParams
        self.rcParamsOrig = rcParamsOrig

    def get_backend(self):
        return 'Agg'

    def set_backend(self, backend):
        pass


mpl = MplModule()