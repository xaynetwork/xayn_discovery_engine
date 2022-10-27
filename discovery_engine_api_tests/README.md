### Installation  🛠
Python 3.8 or higher required
1. `cd engine_api_tests && pip install -r requirements.txt`
2. `export PYTHONPATH=$(pwd)`


### Test execution and reporting 📱
1. `cd tests`
2. Here is the list of available execution commands:
 - `pytest -q --alluredir=$(pwd)/reports` - execute all tests
 - `pytest -q test_interactions_endpoint.py --alluredir=$(pwd)/reports` - execute tests within **test_interactions_endpoint.py**
 - `pytest -q --alluredir=${pwd}/reports --allure-severities normal,critical` - execute test with only **normal** and **critical** severity
 - `pytest -k TestInteractionsEndpoint --alluredir=$(pwd)/reports` - execute all tests in **TestInteractionsEndpoint** class
 - `allure serve $(pwd)/reports` - to generate Allure report

### General rules  📝

 - All test classes should be within `tests` folder
 - All test class as well as test methods names should start with `test_`
 - All test classes should be associated with certain endpoint and all test methods within the class should correspond to it
 - All tests should be marked with label `@allure.severity(allure.severity_level`
 - All testing urls should be within `tests/config.ini` file under corresponding category
 - All tests that require refactoring or expected to skip should be marked with label `@pytest.mark.skip("reason for skipping")`
 - All tests that expected to fail due some bug or improvement should be marked with label `@pytest.mark.xfail("reason for failing")`
