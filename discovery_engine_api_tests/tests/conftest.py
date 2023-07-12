import pytest

from model.documents import documents
from model.interactions.interaction import Interaction
from model.interactions.interactions import Interactions
from utils import assert_utils as au
from utils import test_utils as tu
from handlers.api_handler import ApiHandler


@pytest.fixture
def ingest_generated_document():
    """
    Method that generates one document, ingests it
    :return: id of a doc
    """
    api_handler = ApiHandler()
    doc_dict = documents.generate_docs(1)
    request = api_handler.ingest_document(list(doc_dict.values()))
    au.assert_status_code_equals(request.status_code, 201)
    return doc_dict.popitem()[0]


@pytest.fixture
def ingest_generated_documents_and_interact():
    """
    Method that generates 100 docs, ingests them and interacts with one of them
    :return: id of a user who interacted
    """
    api_handler = ApiHandler()
    user_id = tu.generate_random_alphanumerical(6)
    docs = documents.generate_docs(100)
    request = api_handler.ingest_document(list(docs.values()))
    au.assert_status_code_equals(request.status_code, 201)
    interaction = Interactions(Interaction(id=next(iter(docs)))).to_json()
    request = api_handler.interact_with_documents(user_id, interaction)
    au.assert_status_code_equals(request.status_code, 204)
    return user_id
