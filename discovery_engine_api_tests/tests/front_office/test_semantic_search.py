import allure
from handlers.api_handler import ApiHandler
from utils import assert_utils as au
from utils import test_utils as tu


@allure.suite("Semantic Search Test Suite")
class TestSemanticSearch:
    api_handler = ApiHandler()

    @allure.severity(allure.severity_level.NORMAL)
    def test_semantic_search(self, ingest_generated_document):
        request = self.api_handler.get_semantic_search_doc(doc_id=ingest_generated_document)
        au.assert_status_code_equals(request.status_code, 200)

        data = self.api_handler.deserialize_json(request.text)
        au.assert_score_is_valid(data)

    @allure.severity(allure.severity_level.NORMAL)
    def test_semantic_search_negative(self):
        request = self.api_handler.get_semantic_search_doc(doc_id=tu.generate_random_numbers(20))
        au.assert_status_code_equals(request.status_code, 400)

        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["kind"], "DocumentNotFound")

        request = self.api_handler.get_semantic_search_doc(doc_id=tu.generate_invalid_id())
        au.assert_status_code_equals(request.status_code, 400)

        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["kind"], "InvalidDocumentId")
