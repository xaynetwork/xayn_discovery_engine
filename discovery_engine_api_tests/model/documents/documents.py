from model.base.model_base import ModelBase
from model.properties.property import Property
from model.documents.document import Document
from utils import test_utils as tu


class Documents(ModelBase):

    def __init__(self, docs):
        self.documents = docs


def generate_docs(amount):
    """
    Methods that generates a dict where the key is an id of a doc and value is a doc object itself
    :param amount: amount of docs to be generated
    :return:
    """
    docs_dict = {}
    for i in range(amount):
        id = tu.generate_random_alphanumerical(20)
        snippet = tu.generate_random_alphanumerical(50)
        property = Property(title="Title", publication_date=tu.get_current_date_time())
        doc = Document(id, snippet, property)
        docs_dict[id] = doc
    return docs_dict

