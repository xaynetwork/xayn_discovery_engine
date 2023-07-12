### Installation  🛠
Python 3.8 or higher required
1. `cd discovery_engine_api_tests && pip install -r requirements.txt`
2. `export PYTHONPATH=$(pwd)`
3. `make sure you have INGESTION_URI and PERSONALIZATION_URI in your env variables`


### Test execution and reporting 📱
1. `cd tests`
2. Execution examples:
 - `pytest --maxfail=3` - execute all tests (stop execution after 3 errors or failures)
 - `pytest front_office/test_interactions.py` - execute tests within **test_interactions.py**
 - `pytest --allure-severities normal,critical` - execute test with only **normal** and **critical** severity
 - `pytest -k TestInteractionsEndpoint` - execute all tests in **TestInteractionsEndpoint** class
 - `allure serve $(pwd)/reports` - generate Allure report

### General rules  📝

 - All test classes should be within `tests` folder
 - All test class as well as test methods names should start with `test_`
 - All tests should be marked with label `@allure.severity(allure.severity_level)`
 - All test that contain multiple checks should use soft assertions from assert_utils
 - All reusable setup/teardown methods must be decorated with `@pytest.fixture` (see examples inside tests)
 - All tests that require refactoring or expected to skip should be marked with label `@pytest.mark.skip("reason for skipping")`
 - All tests that expected to fail due some bug or improvement should be marked with label `@pytest.mark.xfail("reason for failing")`
