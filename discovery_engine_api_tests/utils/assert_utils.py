import allure


@allure.step("assert status code from response {var1} equals {var2}")
def assert_status_code_equals(var1, var2):
    assert var1 == var2
