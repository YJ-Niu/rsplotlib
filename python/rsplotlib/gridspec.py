"""rsplotlib.gridspec - Matplotlib GridSpec 兼容接口

提供子图网格布局管理。
底层实现已迁移至 Rust 层，此模块为保持完整 Python 接口的薄包装层。
"""

from . import rsplotlib as _rs


class GridSpec:
    """GridSpec 布局管理器

    用于在 Figure 中创建子图网格布局。
    底层实现: Rust GridSpec

    Args:
        nrows: 行数
        ncols: 列数
        left: 左边界 (0-1)
        bottom: 下边界 (0-1)
        right: 右边界 (0-1)
        top: 上边界 (0-1)
        wspace: 列间距
        hspace: 行间距
        width_ratios: 列宽比例
        height_ratios: 行高比例
    """

    def __init__(self, nrows=1, ncols=1, left=None, bottom=None, right=None,
                 top=None, wspace=None, hspace=None, width_ratios=None,
                 height_ratios=None):
        self.nrows = nrows
        self.ncols = ncols
        self.left = left
        self.bottom = bottom
        self.right = right
        self.top = top
        self.wspace = wspace
        self.hspace = hspace
        self.width_ratios = width_ratios or [1] * ncols
        self.height_ratios = height_ratios or [1] * nrows
        self._impl = _rs.GridSpec(nrows, ncols, left, bottom, right, top,
                                   wspace, hspace, width_ratios, height_ratios)

    def __getitem__(self, key):
        """支持 gs[row, col] 和 gs[row_start:row_end, col_start:col_end] 语法

        底层实现: Rust GridSpec.__getitem__

        Returns:
            SubplotSpec: 子图定位器
        """
        result = self._impl.__getitem__(key)
        # 将 Rust SubplotSpec 包装为 Python SubplotSpec
        return SubplotSpec._from_rust(result)

    def get_subplot_params(self, figure=None):
        """获取子图布局参数"""
        return {
            'left': self.left,
            'bottom': self.bottom,
            'right': self.right,
            'top': self.top,
            'wspace': self.wspace,
            'hspace': self.hspace,
        }

    def tight_layout(self, figure=None, renderer=None, pad=1.08, h_pad=None, w_pad=None, rect=None):
        """自动调整布局"""
        self._impl.tight_layout(figure, renderer)


class SubplotSpec:
    """子图定位器

    底层实现: Rust SubplotSpec
    """

    def __init__(self, gridspec, row_start=0, row_end=1, col_start=0, col_end=1):
        self.gridspec = gridspec
        self.row_start = row_start
        self.row_end = row_end
        self.col_start = col_start
        self.col_end = col_end
        if gridspec is not None:
            self.numRows = gridspec.nrows
            self.numCols = gridspec.ncols
        else:
            self.numRows = max(row_end, 1)
            self.numCols = max(col_end, 1)
        self.rowStart = row_start
        self.rowStop = row_end
        self.colStart = col_start
        self.colStop = col_end
        # 创建 Rust 实现
        if gridspec is not None:
            self._impl = _rs.SubplotSpec(gridspec._impl, row_start, row_end, col_start, col_end)
        else:
            self._impl = _rs.SubplotSpec(None, row_start, row_end, col_start, col_end)

    @classmethod
    def _from_rust(cls, rust_spec):
        """从 Rust SubplotSpec 创建 Python 包装"""
        instance = cls.__new__(cls)
        instance._impl = rust_spec
        instance.numRows = rust_spec.numRows
        instance.numCols = rust_spec.numCols
        instance.rowStart = rust_spec.rowStart
        instance.rowStop = rust_spec.rowStop
        instance.colStart = rust_spec.colStart
        instance.colStop = rust_spec.colStop
        instance.row_start = rust_spec.rowStart
        instance.row_end = rust_spec.rowStop
        instance.col_start = rust_spec.colStart
        instance.col_end = rust_spec.colStop
        instance.gridspec = None  # 简化处理
        return instance

    def get_position(self, figure):
        """返回子图位置 (left, bottom, width, height)

        底层实现: Rust SubplotSpec.get_position
        """
        return self._impl.get_position(figure)

    def get_grid_span(self):
        """返回网格跨度"""
        return (self.rowStart, self.rowStop, self.colStart, self.colStop)


# 便利函数
def GridSpecFromSubplotSpec(nrows, ncols, subplot_spec, **kwargs):
    """从 SubplotSpec 创建 GridSpec"""
    from .rsplotlib import gridspec_from_subplotspec
    # 参数提取
    left = kwargs.get('left')
    bottom = kwargs.get('bottom')
    right = kwargs.get('right')
    top = kwargs.get('top')
    wspace = kwargs.get('wspace')
    hspace = kwargs.get('hspace')
    width_ratios = kwargs.get('width_ratios')
    height_ratios = kwargs.get('height_ratios')
    rust_gs = gridspec_from_subplotspec(
        nrows, ncols, left=left, bottom=bottom, right=right, top=top,
        wspace=wspace, hspace=hspace, width_ratios=width_ratios,
        height_ratios=height_ratios,
    )
    # 包装为 Python GridSpec
    gs = GridSpec.__new__(GridSpec)
    gs.nrows = nrows
    gs.ncols = ncols
    gs.left = left
    gs.bottom = bottom
    gs.right = right
    gs.top = top
    gs.wspace = wspace
    gs.hspace = hspace
    gs.width_ratios = width_ratios or [1] * ncols
    gs.height_ratios = height_ratios or [1] * nrows
    gs._impl = rust_gs
    return gs