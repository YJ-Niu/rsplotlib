"""rsplotlib.gridspec - Matplotlib GridSpec 兼容接口"""


class GridSpec:
    """GridSpec 布局管理器
    
    用于在 Figure 中创建子图网格布局。
    
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

    def __getitem__(self, key):
        """支持 gs[row, col] 和 gs[row_start:row_end, col_start:col_end] 语法
        
        Returns:
            SubplotSpec: 子图定位器
        """
        if isinstance(key, tuple):
            row_spec, col_spec = key
            row_start, row_end = self._parse_slice(row_spec)
            col_start, col_end = self._parse_slice(col_spec)
            return SubplotSpec(self, row_start, row_end, col_start, col_end)
        raise TypeError("GridSpec indices must be tuples (row, col)")

    def _parse_slice(self, spec):
        if isinstance(spec, slice):
            start = spec.start or 0
            stop = spec.stop or (self.nrows if isinstance(spec, slice) else self.ncols)
            return start, stop
        return spec, spec + 1

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
        pass


class SubplotSpec:
    """子图定位器"""

    def __init__(self, gridspec, row_start, row_end, col_start, col_end):
        self.gridspec = gridspec
        self.row_start = row_start
        self.row_end = row_end
        self.col_start = col_start
        self.col_end = col_end
        if gridspec is not None:
            self.numRows = gridspec.nrows
            self.numCols = gridspec.ncols
        else:
            # 用于模式 (nrows, ncols, index) 调用 add_subplot
            self.numRows = max(row_end, 1)
            self.numCols = max(col_end, 1)
        self.rowStart = row_start
        self.rowStop = row_end
        self.colStart = col_start
        self.colStop = col_end

    def get_position(self, figure):
        """返回子图位置 (left, bottom, width, height)"""
        if self.gridspec is None:
            # 没有 gridspec 时使用均匀划分
            return (self.colStart / self.numCols,
                    1.0 - self.rowEnd / self.numRows,
                    (self.colEnd - self.colStart) / self.numCols,
                    (self.rowEnd - self.rowStart) / self.numRows)
        row_heights = self.gridspec.height_ratios
        col_widths = self.gridspec.width_ratios

        total_h = sum(row_heights)
        total_w = sum(col_widths)

        x = sum(col_widths[:self.col_start]) / total_w
        y = 1.0 - sum(row_heights[:self.row_end]) / total_h
        w = sum(col_widths[self.col_start:self.col_end]) / total_w
        h = sum(row_heights[self.row_start:self.row_end]) / total_h

        return x, y, w, h

    def get_grid_span(self):
        """返回网格跨度"""
        return (self.row_start, self.row_end, self.col_start, self.col_end)


# 便利函数
def GridSpecFromSubplotSpec(nrows, ncols, subplot_spec, **kwargs):
    """从 SubplotSpec 创建 GridSpec"""
    return GridSpec(nrows, ncols, **kwargs)
