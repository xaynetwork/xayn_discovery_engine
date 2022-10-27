from model.base.model_base import ModelBase
from model.properties import Properties
from utils import string_utils as su


class Documents(ModelBase):

    def __init__(self, id, snippet, props):
        self.documents = [{"id": id, "snippet": snippet, "properties": props}]


def generate_docs(amount):
    docs_dict = {}
    for i in range(amount):
        id = su.generate_random_alphanumerical(10)
        snippet = su.generate_random_alphanumerical(50)
        properties = Properties("Title")
        doc = Documents(id, snippet, properties).toJSON()
        docs_dict[id] = doc
    return docs_dict

