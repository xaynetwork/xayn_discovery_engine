import allure
import pytest
from handlers.api_handler import ApiHandler
from model.interactions.interactions import Interactions
from model.interactions.interaction import Interaction
from model.documents import documents
from utils import test_utils as su
from utils import assert_utils as au


@allure.suite("Front Office Test Suite")
class TestFrontOffice:
    api_handler = ApiHandler()

    @pytest.fixture
    def ingest_generated_documents(self):
        doc_dict = documents.generate_docs(1).popitem()
        request = self.api_handler.ingest_document(doc_dict[1])
        au.assert_status_code_equals(request.status_code, 201)
        return doc_dict[0]

    @allure.severity(allure.severity_level.CRITICAL)
    def test_positive_interaction(self, ingest_generated_documents):
        user_id = su.generate_random_alphanumerical(6)
        positive_interaction = Interactions(Interaction(id=ingest_generated_documents, type="Positive")).to_json()
        request = self.api_handler.interact_with_documents(user_id, positive_interaction)
        au.assert_status_code_equals(request.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    @pytest.mark.skip(reason="todo: need to be updated")
    def test_invalid_doc_id(self):
        user_id = su.generate_random_alphanumerical(6)
        interaction_with_random_id = Interactions(su.generate_random_alphanumerical(36), "Positive").to_json()
        request = self.api_handler.interact_with_documents(user_id, interaction_with_random_id)
        assert request.status_code == 400

    @allure.severity(allure.severity_level.NORMAL)
    def test_with_null_values(self, ingest_generated_documents):
        user_id = su.generate_random_alphanumerical(6)
        interaction_with_null_id = Interactions(Interaction(id=None, type="Positive")).to_json()
        interaction_with_null_type = Interactions(Interaction(id=ingest_generated_documents, type=None)).to_json()
        request_null_id = self.api_handler.interact_with_documents(user_id, interaction_with_null_id)
        request_null_type = self.api_handler.interact_with_documents(user_id, interaction_with_null_type)
        au.soft_assert_status_code_equals(request_null_id.status_code, 400)
        au.soft_assert_status_code_equals(request_null_type.status_code, 400)
