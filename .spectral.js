const ibmCloudValidationRules = require('@ibm-cloud/openapi-ruleset');
const { enumCaseConvention, pathSegmentCaseConvention } = require('@ibm-cloud/openapi-ruleset/src/functions');
const { paths, schemas } = require('@ibm-cloud/openapi-ruleset/src/collections');

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
          type: 'pascal',
        },
      },
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
          type: 'pascal',
        },
      },
    },
    'path-segment-case-convention': {
      description: 'Path segments must be snake case',
      message: '{{error}}',
      resolved: true,
      given: paths,
      severity: 'error',
      then: {
        function: pathSegmentCaseConvention,
        functionOptions: {
          type: 'snake',
          separator: {
            char: '_',
            allowLeading: true
          }
        },
      },
    },
    'major-version-in-path': 'off',
    'schema-description': 'off',
    'inline-property-schema': 'off',
    'property-description': 'off',
    'oas3-api-servers': 'off',
    'delete-body': 'off',
    'prohibit-summary-sentence-style': 'off',
    'collection-array-property': 'off',
    // the rule set wants to enforce a specific error schema
    'response-error-response-schema': 'off',
    'patch-request-content-type': 'off',
  },
  overrides: [
    {
      files: [
        "web-api/openapi/back_office.yaml#/components/schemas/DocumentCandidatesRequest",
        "web-api/openapi/back_office.yaml#/components/schemas/DocumentCandidatesResponse",
      ],
      rules: {
        "array-boundary": "off",
      },
    },
  ],
};
