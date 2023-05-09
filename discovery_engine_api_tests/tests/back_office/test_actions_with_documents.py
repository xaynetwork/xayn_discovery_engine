import allure
import pytest

from handlers.api_handler import ApiHandler
from model.properties.properties import Properties
from model.properties.property import Property
from utils import assert_utils as au
from utils import test_utils as tu


@allure.suite("Test Actions With Documents Test Suite")
class TestActionsWithDocuments:
    api_handler = ApiHandler()

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc(self, ingest_generated_document):
        request = self.api_handler.delete_document(doc_id=ingest_generated_document)
        au.assert_status_code_equals(request.status_code, 204)

    @allure.severity(allure.severity_level.NORMAL)
    def test_delete_doc_negative(self):
        request = self.api_handler.delete_document(doc_id=tu.generate_random_alphanumerical(10))
        au.assert_status_code_equals(request.status_code, 400)

    @allure.severity(allure.severity_level.NORMAL)
    def test_get_property(self, ingest_generated_document):
        request_get_property = self.api_handler.get_properties(doc_id=ingest_generated_document)
        au.assert_status_code_equals(request_get_property.status_code, 200)

        data = self.api_handler.deserialize_json(request_get_property.text)
        au.assert_strings_equal(data["properties"]["title"], "Title")

    @allure.severity(allure.severity_level.NORMAL)
    def test_replace_property(self, ingest_generated_document):
        updated_property = Properties(Property(title="Title2")).to_json()
        request_updated_property = self.api_handler.set_property(doc_id=ingest_generated_document,
                                                                 properties=updated_property)
        au.assert_status_code_equals(request_updated_property.status_code, 204)

        request_get_updated_property = self.api_handler.get_properties(doc_id=ingest_generated_document)
        data = self.api_handler.deserialize_json(request_get_updated_property.text)
        au.assert_strings_equal(data["properties"]["title"], "Title2")
