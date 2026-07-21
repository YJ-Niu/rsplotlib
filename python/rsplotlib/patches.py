class FancyArrowPatch:
    def __init__(self, posA, posB, arrowstyle=None, connectionstyle=None,
                 color=None, linestyle=None, linewidth=None, mutation_scale=None,
                 shrinkA=None, shrinkB=None, zorder=None):
        self.posA = posA
        self.posB = posB
        self.arrowstyle = arrowstyle
        self.connectionstyle = connectionstyle
        self.color = color
        self.linestyle = linestyle
        self.linewidth = linewidth
        self.mutation_scale = mutation_scale
        self.shrinkA = shrinkA if shrinkA is not None else 0.0
        self.shrinkB = shrinkB if shrinkB is not None else 0.0
        self.zorder = zorder
        self._dpi_cor = 1.0
        self._posA_posB = (posA, posB)
        self.patchA = None
        self.patchB = None

    def _convert_xy_units(self, xy):
        return xy

    def get_connectionstyle(self):
        return self.connectionstyle


class _Path:
    def __init__(self, vertices, codes=None):
        self.vertices = vertices
        self.codes = codes


class _ConnectionStyleType:
    pass


class ConnectionStyle(_ConnectionStyleType):
    def __init__(self, style):
        self.style = style

    def __call__(self, posA, posB, *args, **kwargs):
        x1, y1 = posA
        x2, y2 = posB
        cx = (x1 + x2) / 2
        cy = (y1 + y2) / 2
        return _Path([posA, (cx, cy), posB], [1, 2, 2])


ConnectionStyle.Angle3 = _ConnectionStyleType
ConnectionStyle.Arc3 = _ConnectionStyleType
ConnectionStyle.Angle = _ConnectionStyleType
ConnectionStyle.Arc = _ConnectionStyleType
ConnectionStyle.Bar = _ConnectionStyleType
