import allure
import pytest
from handlers.api_handler import ApiHandler
from model.interactions import Interactions
from model import documents
from utils import test_utils as su
from utils import assert_utils as au


@allure.suite("Front Office Test Suite")
class TestFrontOffice:
    api_handler = ApiHandler()

    @pytest.fixture
    def generate_post_docs(self):
        doc_dict = documents.generate_docs(1).popitem()
        request = self.api_handler.post_document(doc_dict[1])
        au.assert_status_code_equals(request.status_code, 201)
        return doc_dict[0]

    @allure.severity(allure.severity_level.CRITICAL)
    def test_positive_interaction(self, generate_post_docs):
        user_id = su.generate_random_alphanumerical(6)
        positive_interaction = Interactions(generate_post_docs, "Positive").to_json()
        request = self.api_handler.interact_with_documents(user_id, positive_interaction)
        au.assert_status_code_equals(request.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: create ticket")
    def test_invalid_doc_id(self):
        user_id = su.generate_random_alphanumerical(6)
        interaction_with_random_id = Interactions(su.generate_random_alphanumerical(36), "Positive").to_json()
        request = self.api_handler.interact_with_documents(user_id, interaction_with_random_id)
        assert request.status_code == 400

    @allure.severity(allure.severity_level.NORMAL)
    def test_with_null_values(self, generate_post_docs):
        user_id = su.generate_random_alphanumerical(6)
        interaction_with_null_id = Interactions(None, "Positive").to_json()
        interaction_with_null_type = Interactions(generate_post_docs, None).to_json()
        request_null_id = self.api_handler.interact_with_documents(user_id, interaction_with_null_id)
        request_null_type = self.api_handler.interact_with_documents(user_id, interaction_with_null_type)
        au.soft_assert_status_code_equals(request_null_id.status_code, 400)
        au.soft_assert_status_code_equals(request_null_type.status_code, 400)
