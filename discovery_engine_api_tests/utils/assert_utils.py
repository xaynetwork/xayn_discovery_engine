import allure
import pytest_check as soft_check
from datetime import datetime
from model.documents.documents import Document


@allure.step("assert status code from response {var1} equals {var2}")
def assert_status_code_equals(var1, var2):
    assert var1 == var2, f"status code from response {var1} must be equal {var2}"


@allure.step("soft assert status code from response {var1} equals {var2}")
def soft_assert_status_code_equals(var1, var2):
    soft_check.equal(var1, var2, f"status code from response {var1} must be equal {var2}")


@allure.step("assert string {var1} equals string {var2}")
def assert_strings_equal(var1, var2):
    assert var1 == var2, f"string {var1} must be equal {var2}"


"""
Document specific checks
"""


@allure.step("assert that score is valid")
def assert_score_is_valid(docs):
    array_of_docs = docs["documents"]
    for doc in array_of_docs:
        assert 0 <= float(doc["score"]) <= 1, f"score must be between 0 and 1"


@allure.step("assert that document field is valid")
def assert_documents_are_valid(docs):
    array_of_docs = docs["documents"]
    assert len(array_of_docs) > 0, f"Array of docs shouldn't be empty"
    for doc in array_of_docs:
        assert all(key in doc for key in ['id', 'score', 'properties']), f"all documents should contain specific " \
                                                                         f"fields such as 'id', 'score','properties' "

@allure.step("assert that publication date is valid")
def assert_date_is_after(docs, after_date):
    array_of_docs = docs["documents"]
    assert len(array_of_docs) > 0, f"Array of docs shouldn't be empty"
    for doc in array_of_docs:
        publication_date = datetime.fromisoformat(doc['properties']['publication_date'])
        assert publication_date > after_date, f"Publication date {publication_date} is not after {after_date}"
