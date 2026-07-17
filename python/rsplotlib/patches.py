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
        self.shrinkA = shrinkA
        self.shrinkB = shrinkB
        self.zorder = zorder
