
import allure
import pytest
from handlers.api_handler import ApiHandler
from model.documents import documents
from utils import test_utils as su
from utils import assert_utils as au


@allure.suite("Back Office Test Suite")
class TestBackOfficeEndpoint:
    api_handler = ApiHandler()

    @pytest.fixture
    def ingest_generated_documents(self):
        doc_dict = documents.generate_docs(1).popitem()
        request = self.api_handler.ingest_document(doc_dict[1])
        au.assert_status_code_equals(request.status_code, 201)
        return doc_dict[0]

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc(self, ingest_generated_documents):
        docs = self.api_handler.delete_document(ingest_generated_documents)
        au.assert_status_code_equals(docs.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: create ticket")
    def test_delete_doc_negative(self):
        docs = self.api_handler.delete_document(su.generate_random_alphanumerical(10))
        au.assert_status_code_equals(docs.status_code, 400)

    @allure.severity(allure.severity_level.NORMAL)
    def test_get_property(self, ingest_generated_documents):
        request = self.api_handler.get_properties(ingest_generated_documents)
        au.assert_status_code_equals(request.status_code, 200)
        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["properties"]["title"], "Title")

