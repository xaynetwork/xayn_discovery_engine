from model.base.model_base import ModelBase

class Interaction(ModelBase):
    def __init__(self, **kwargs):
        for key, value in kwargs.items():
            setattr(self, key, value)
