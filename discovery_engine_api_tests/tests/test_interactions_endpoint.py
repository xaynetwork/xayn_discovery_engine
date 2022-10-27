import allure
import pytest

from handlers.api_handler import ApiHandler
from model.documents import Documents
from model import documents
from model.interactions import Interactions
from model.properties import Properties
from utils import string_utils as su
from utils import assert_utils as au


@allure.suite("Interactions Test Suite")
class TestInteractionsEndpoint:

    @pytest.fixture
    def post_docs(self):
        return ApiHandler().post_documents(documents.generate_docs(1))

    @allure.severity(allure.severity_level.CRITICAL)
    def test_positive_interaction(self, post_docs):
        data = Interactions(post_docs[0], interaction_type="positive").toJSON()
        request = ApiHandler().interact_with_documents(data)
        au.assert_status_code_equals(request.status_code, 204)

    @pytest.mark.skip("ET-3349 not implemented yet")
    @allure.severity(allure.severity_level.NORMAL)
    def test_negative_interaction(self, post_docs):
        data = Interactions(post_docs[0], interaction_type="negative").toJSON()
        request = ApiHandler().interact_with_documents(data)
        au.assert_status_code_equals(request.status_code, 204)

    @allure.severity(allure.severity_level.MINOR)
    def test_invalid_doc(self):
        data = Interactions(id=su.generate_random_alphanumerical(36), interaction_type="positive").toJSON()
        request = ApiHandler().interact_with_documents(data)
        assert request.status_code == 400

    @allure.severity(allure.severity_level.MINOR)
    def test_invalid_reaction(self, post_docs):
        data = Interactions(id=post_docs[0], interaction_type=su.generate_random_letters(10)).toJSON()
        request = ApiHandler().interact_with_documents(data)
        assert request.status_code == 400

    @allure.severity(allure.severity_level.MINOR)
    def test_invalid_user_id(self):
        data = Interactions(su.generate_random_numbers(10), interaction_type="positive").toJSON()
        request = ApiHandler().interact_with_documents(data)
        assert request.status_code == 400
