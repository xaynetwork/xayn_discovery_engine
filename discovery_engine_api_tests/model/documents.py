from model.base.model_base import ModelBase
from model.properties import Properties
from utils import test_utils as su


class Documents(ModelBase):

    def __init__(self, id, snippet, props):
        self.documents = [{"id": id, "snippet": snippet, "properties": props}]


def generate_docs(amount):
    """
    Methods that generates a dict where the key is an id of a doc and value is a doc object itself
    :param amount: amount of docs to be generated
    :return:
    """
    docs_dict = {}
    for i in range(amount):
        id = su.generate_random_alphanumerical(10)
        snippet = su.generate_random_alphanumerical(50)
        properties = Properties("Title")
        doc = Documents(id, snippet, properties).to_json()
        docs_dict[id] = doc
    return docs_dict

