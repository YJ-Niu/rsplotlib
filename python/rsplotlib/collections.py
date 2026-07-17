class PathCollection:
    def __init__(self, paths):
        self.paths = paths


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
