from model.base.model_base import ModelBase


class Interactions(ModelBase):
    def __init__(self, *docs):
        self.documents = []
        for doc in docs:
            self.documents.append(doc)

