const ibmCloudValidationRules = require('@ibm-cloud/openapi-ruleset');
const { enumCasingConvention, pathSegmentCasingConvention } = require('@ibm-cloud/openapi-ruleset/src/functions');
const { operations, paths, schemas } = require('@ibm-cloud/openapi-ruleset-utilities/src/collections');

module.exports = {
  extends: ibmCloudValidationRules,
  rules: {
    'ibm-enum-casing-convention': {
      description: 'Enum names must be pascal case',
      message: '{{error}}',
      resolved: true,
      given: schemas,
      severity: 'error',
      then: {
        function: enumCasingConvention,
        functionOptions: {
          type: 'pascal',
        },
      },
    },
    'ibm-operationid-casing-convention': {
      description: 'Operation ids must be pascal case',
      message: '{{error}}',
      resolved: true,
      given: operations,
      severity: 'error',
      then: {
        function: enumCasingConvention,
        functionOptions: {
          type: 'pascal',
        },
      },
    },
    'ibm-path-segment-casing-convention': {
      description: 'Path segments must be snake case',
      message: '{{error}}',
      resolved: true,
      given: paths,
      severity: 'error',
      then: {
        function: pathSegmentCasingConvention,
        functionOptions: {
          type: 'snake',
          separator: {
            char: '_',
            allowLeading: true
          }
        },
      },
    },
    'ibm-major-version-in-path': 'off',
    'ibm-schema-description': 'off',
    // spectral has its own rules to check schemas that support v3.1
    'ibm-schema-type-format': 'off',
    'ibm-avoid-inline-schemas': 'off',
    'ibm-property-description': 'off',
    'oas3-api-servers': 'off',
    'ibm-no-body-for-delete': 'off',
    'ibm-summary-sentence-style': 'off',
    'ibm-collection-array-property': 'off',
    // the rule set wants to enforce a specific error schema
    'ibm-error-response-schemas': 'off',
    'ibm-patch-request-content-type': 'off',
  },
  overrides: [
    {
      files: [
        "web-api/openapi/back_office.yaml#/components/schemas/DocumentCandidatesRequest",
        "web-api/openapi/back_office.yaml#/components/schemas/DocumentCandidatesResponse",
      ],
      rules: {
        "ibm-array-attributes": "off",
      },
    },
    {
      files: [
        "web-api/openapi/schemas/document.yml#/IndexedPropertyType.enum[0]",
        "web-api/openapi/schemas/document.yml#/IndexedPropertyType.enum[1]",
        "web-api/openapi/schemas/document.yml#/IndexedPropertyType.enum[2]",
        "web-api/openapi/schemas/document.yml#/IndexedPropertyType.enum[3]",
        "web-api/openapi/schemas/document.yml#/IndexedPropertyType.enum[4]",
      ],
      rules: {
        "ibm-enum-casing-convention": "off",
      }
    },
  ],
};
