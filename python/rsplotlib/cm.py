class ScalarMappable:
    def __init__(self, cmap=None, norm=None):
        self.cmap = cmap
        self.norm = norm
        self.vmin = None
        self.vmax = None

    def set_clim(self, vmin=None, vmax=None):
        self.vmin = vmin
        self.vmax = vmax

    def to_rgba(self, values):
        if not isinstance(values, (list, tuple)):
            values = [values]
        return [[0.0, 0.0, 0.0, 1.0] for _ in values]
