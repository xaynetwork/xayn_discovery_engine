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
    'string-boundary': 'off',
    'array-boundary': 'off',
    'schema-description': 'off',
    'inline-property-schema': 'off',
    'content-entry-provided': 'off',
    'property-description': 'off',
    'oas3-api-servers': 'off',
    'operation-id-naming-convention': 'off',
    'delete-body': 'off',
    'optional-request-body': 'off',
    'prohibit-summary-sentence-style': 'off',
    'response-status-codes': 'off',
    'response-example-provided': 'off',
    'collection-array-property': 'off',
    'response-error-response-schema': 'off',
    'enum-case-convention': 'off',
    'operation-id-case-convention': 'off',
    'patch-request-content-type': 'off',
  }
};
