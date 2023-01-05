const ibmCloudValidationRules = require('@ibm-cloud/openapi-ruleset');
const { enumCaseConvention, operationIdCaseConvention } = require('@ibm-cloud/openapi-ruleset/src/functions');
const { schemas } = require('@ibm-cloud/openapi-ruleset/src/collections');

module.exports = {
  extends: ibmCloudValidationRules,
  rules: {
    'enum-case-convention': {
      description: 'enum names must follow pascal case',
      message: '{{error}}',
      resolved: true,
      given: schemas,
      severity: 'error',
      then: {
        function: enumCaseConvention,
        functionOptions: {
          type: 'pascal'
        }
      }
    },
    'operation-id-case-convention': {
      description: 'Operation ids must be pascal case',
      message: '{{error}}',
      resolved: true,
      given: schemas,
      severity: 'error',
      then: {
        function: enumCaseConvention,
        functionOptions: {
          type: 'pascal'
        }
      }
    },
    'major-version-in-path': 'off',
    'schema-description': 'off',
    'inline-property-schema': 'off',
    'property-description': 'off',
    'oas3-api-servers': 'off',
    'delete-body': 'off',
    'prohibit-summary-sentence-style': 'off',
    'collection-array-property': 'off',
    // the rule set wants to enforce a specific erorr schema
    'response-error-response-schema': 'off',
    'patch-request-content-type': 'off',
  }
};
