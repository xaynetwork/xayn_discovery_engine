import allure
import pytest_check as soft_check


@allure.step("assert status code from response {var1} equals {var2}")
def assert_status_code_equals(var1, var2):
    assert var1 == var2, f"assert status code from response {var1} equals {var2}"


@allure.step("soft assert status code from response {var1} equals {var2}")
def soft_assert_status_code_equals(var1, var2):
    soft_check.equal(var1, var2, f"soft assert status code from response {var1} equals {var2}")


@allure.step("assert string {var1} equals string {var2}")
def assert_strings_equal(var1, var2):
    assert var1 == var2, f"assert string {var1} equals {var2}"
