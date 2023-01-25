
import allure
import pytest
from handlers.api_handler import ApiHandler
from model import documents
from utils import test_utils as su
from utils import assert_utils as au


@allure.suite("Back Office Test Suite")
class TestBackOfficeEndpoint:
    api_handler = ApiHandler()

    @pytest.fixture
    def generate_post_docs(self):
        doc_dict = documents.generate_docs(1).popitem()
        request = self.api_handler.post_document(doc_dict[1])
        au.assert_status_code_equals(request.status_code, 201)
        return doc_dict[0]

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc(self, generate_post_docs):
        docs = self.api_handler.delete_document(generate_post_docs)
        au.assert_status_code_equals(docs.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: create ticket")
    def test_delete_doc_negative(self):
        docs = self.api_handler.delete_document(su.generate_random_alphanumerical(10))
        au.assert_status_code_equals(docs.status_code, 400)

    @allure.severity(allure.severity_level.NORMAL)
    def test_get_property(self, generate_post_docs):
        request = self.api_handler.get_properties(generate_post_docs)
        au.assert_status_code_equals(request.status_code, 200)
        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["properties"]["title"], "Title")

