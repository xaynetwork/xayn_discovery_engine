from model.base.model_base import ModelBase


class Interactions(ModelBase):
    def __init__(self, id, interaction_type):
        self.documents = [{"id": id, "type": interaction_type}]
