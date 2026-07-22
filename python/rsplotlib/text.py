class Text:
    def __init__(self, x=0, y=0, text='', **kwargs):
        self.x = x
        self.y = y
        self.text = text
        self.rotation = kwargs.get('rotation', 0)
        self.fontsize = kwargs.get('fontsize', kwargs.get('size', None))
        self.color = kwargs.get('color', kwargs.get('c', None))
        self.label = kwargs.get('label', None)
        self.horizontalalignment = kwargs.get('horizontalalignment', 'center')
        self.verticalalignment = kwargs.get('verticalalignment', 'center')
        self._rendered = False

    def set_position(self, pos):
        self.x, self.y = pos

    def set_rotation(self, rotation):
        self.rotation = rotation
