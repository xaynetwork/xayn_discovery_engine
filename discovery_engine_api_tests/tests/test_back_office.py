import allure
import pytest

from handlers.api_handler import ApiHandler
from model.properties.properties import Properties
from model.properties.property import Property
from utils import assert_utils as au
from utils import test_utils as su


@allure.suite("Back Office Test Suite")
class TestBackOfficeEndpoint:

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc(self, ingest_generated_documents):
        api_handler = ApiHandler()
        docs = api_handler.delete_document(ingest_generated_documents)
        au.assert_status_code_equals(docs.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc_negative(self):
        api_handler = ApiHandler()
        docs = api_handler.delete_document(su.generate_random_alphanumerical(10))
        au.assert_status_code_equals(docs.status_code, 400)

    @allure.severity(allure.severity_level.NORMAL)
    def test_get_property(self, ingest_generated_documents):
        api_handler = ApiHandler()
        request = api_handler.get_properties(ingest_generated_documents)
        au.assert_status_code_equals(request.status_code, 200)
        data = api_handler.deserialize_json(request.text)
        au.assert_strings_equal(data["properties"]["title"], "Title")

    @allure.severity(allure.severity_level.NORMAL)
    def test_replace_property(self, ingest_generated_documents):
        api_handler = ApiHandler()
        request_get_property = api_handler.get_properties(ingest_generated_documents)
        au.assert_status_code_equals(request_get_property.status_code, 200)
        data = api_handler.deserialize_json(request_get_property.text)
        au.assert_strings_equal(data["properties"]["title"], "Title")

        updated_property = Properties(Property(title="Title2")).to_json()
        request_updated_property = api_handler.set_property(ingest_generated_documents, updated_property)
        au.assert_status_code_equals(request_updated_property.status_code, 204)

        request_get_updated_property = api_handler.get_properties(ingest_generated_documents)
        data = api_handler.deserialize_json(request_get_updated_property.text)
        au.assert_strings_equal(data["properties"]["title"], "Title2")
