
import allure
import pytest
from handlers.api_handler import ApiHandler
from model.interactions.interactions import Interactions
from model.interactions.interaction import Interaction
from utils import test_utils as tu
from utils import assert_utils as au


@allure.suite("Interactions Test Suite")
class TestInteractions:
    api_handler = ApiHandler()

    @allure.severity(allure.severity_level.CRITICAL)
    def test_interaction(self, ingest_generated_document):
        user_id = tu.generate_random_alphanumerical(6)
        interaction = Interactions(Interaction(id=ingest_generated_document)).to_json()
        request = self.api_handler.interact_with_documents(user_id, interaction)
        au.assert_status_code_equals(request.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: need to be updated")
    def test_invalid_doc_id(self):
        user_id = tu.generate_random_alphanumerical(6)
        interaction_with_random_id = Interactions(tu.generate_random_alphanumerical(36)).to_json()
        request = self.api_handler.interact_with_documents(user_id, interaction_with_random_id)
        assert request.status_code == 400

    @allure.severity(allure.severity_level.NORMAL)
    def test_invalid_user_id(self, ingest_generated_document):
        user_id = tu.generate_invalid_id()
        interaction = Interactions(Interaction(ingest_generated_document)).to_json()
        request = self.api_handler.interact_with_documents(user_id, interaction)
        au.assert_status_code_equals(request.status_code, 400)

        data = self.api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["kind"], "InvalidUserId")

    @allure.severity(allure.severity_level.NORMAL)
    def test_interaction_with_null_values(self, ingest_generated_document):
        user_id = tu.generate_random_alphanumerical(6)
        interaction_with_null_id = Interactions(Interaction(id=None)).to_json()
        request_null_id = self.api_handler.interact_with_documents(user_id, interaction_with_null_id)
        au.soft_assert_status_code_equals(request_null_id.status_code, 400)
