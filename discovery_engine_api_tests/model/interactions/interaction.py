from model.base.model_base import ModelBase


class Interaction(ModelBase):
    def __init__(self, id, type):
        self.id = id
        self.type = type
