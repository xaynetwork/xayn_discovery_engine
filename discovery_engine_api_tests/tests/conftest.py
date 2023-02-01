import logging

import pytest

from model.documents import documents
from utils import assert_utils as au
from handlers.api_handler import ApiHandler

LOGGER = logging.getLogger(__name__)


@pytest.fixture
def ingest_generated_documents():
    api_handler = ApiHandler()
    doc_dict = documents.generate_docs(1).popitem()
    request = api_handler.ingest_document(doc_dict[1])
    au.assert_status_code_equals(request.status_code, 201)
    return doc_dict[0]
