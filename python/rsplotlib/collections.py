class PathCollection:
    def __init__(self, paths):
        self.paths = paths
        self.zorder = None

    def set_zorder(self, zorder):
        self.zorder = zorder


class LineCollection:
    def __init__(self, segments, colors=None, linewidths=None, linestyle=None,
                 alpha=None, antialiaseds=None, zorder=None):
        self.segments = segments
        self.colors = colors
        self.linewidths = linewidths
        self.linestyle = linestyle
        self.alpha = alpha
        self.antialiaseds = antialiaseds
        self.zorder = zorder
        self.cmap = None
        self.vmin = None
        self.vmax = None

    def set_cmap(self, cmap):
        self.cmap = cmap

    def set_clim(self, vmin, vmax):
        self.vmin = vmin
        self.vmax = vmax

    def set_zorder(self, zorder):
        self.zorder = zorder

    def set_label(self, label):
        self.label = label
