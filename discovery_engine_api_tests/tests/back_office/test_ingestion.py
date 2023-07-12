import allure

from handlers.api_handler import ApiHandler
from model.documents import documents
from utils import assert_utils as au


@allure.suite("Ingestion Test Suite")
class TestIngestion:
    api_handler = ApiHandler()

    @allure.severity(allure.severity_level.CRITICAL)
    def test_ingest_docs(self):
        docs = documents.generate_docs(100)
        request = self.api_handler.ingest_document(doc=list(docs.values()))
        au.assert_status_code_equals(request.status_code, 201)

    @allure.severity(allure.severity_level.CRITICAL)
    def test_ingest_docs_negative(self):
        docs = documents.generate_docs(101)
        request = self.api_handler.ingest_document(doc=list(docs.values()))
        au.assert_status_code_equals(request.status_code, 400)

        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["details"]["message"], "Document batch size exceeded maximum of 100.")
