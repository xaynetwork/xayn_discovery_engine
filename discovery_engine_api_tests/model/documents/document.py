from model.base.model_base import ModelBase


class Document(ModelBase):

    def __init__(self, id, snippet, properties):
        self.id = id
        self.snippet = snippet
        self.properties = properties
