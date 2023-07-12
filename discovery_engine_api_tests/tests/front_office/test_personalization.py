import allure
import pytest
from handlers.api_handler import ApiHandler
from utils import test_utils as su
from utils import assert_utils as au
from utils import test_utils as tu


@allure.suite("Personalization Test Suite")
class TestPersonalization:
    api_handler = ApiHandler()

    @allure.severity(allure.severity_level.CRITICAL)
    def test_personalization(self, ingest_generated_documents_and_interact):
        user_id = ingest_generated_documents_and_interact
        request_docs = self.api_handler.get_personalized_docs(user_id)
        au.assert_status_code_equals(request_docs.status_code, 200)

        documents_response = self.api_handler.deserialize_json(request_docs.text)
        au.assert_documents_are_valid(documents_response)

    @allure.severity(allure.severity_level.NORMAL)
    def test_invalid_user_id(self):
        user_id = su.generate_invalid_id()
        request_docs = self.api_handler.get_personalized_docs(user_id)
        au.assert_status_code_equals(request_docs.status_code, 400)

        documents_response = self.api_handler.deserialize_json(request_docs.text)
        au.assert_strings_equal(documents_response["kind"], "InvalidUserId")

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: need to be updated")
    def test_personalization_with_published_param(self, ingest_generated_documents_and_interact):
        user_id = ingest_generated_documents_and_interact
        date_and_time_in_past = tu.get_updated_date_time(-1)
        request_docs = self.api_handler.get_personalized_docs(user_id=user_id,
                                                              published=date_and_time_in_past)
        au.assert_status_code_equals(request_docs.status_code, 200)

        documents_response = self.api_handler.deserialize_json(request_docs.text)
        au.assert_documents_are_valid(documents_response)
        au.assert_date_is_after(documents_response, date_and_time_in_past)
