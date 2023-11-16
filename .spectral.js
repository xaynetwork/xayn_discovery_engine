const ibmCloudValidationRules = require('@ibm-cloud/openapi-ruleset');
const { enumCasingConvention, pathSegmentCasingConvention } = require('@ibm-cloud/openapi-ruleset/src/functions');
const { operations, paths, schemas } = require('@ibm-cloud/openapi-ruleset-utilities/src/collections');

module.exports = {
  extends: ibmCloudValidationRules,
  rules: {
    // In json-schema enum is used for any kind of "one of this string values"
    // situations, which is not limited to verbatim rust enum variant names.
    // Additionally linter overrides only work for types defined in the root
    // document. So we can't declare exceptions for the cases where it's not
    // a verbatim rust enum variant name.
    'ibm-enum-casing-convention': "off",
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
        "web-api/openapi/schemas/document.yml#/DocumentPropertyArrayString",
      ],
      rules: {
        "ibm-array-attributes": "off",
      },
    },
    {
      files: [
        "web-api/openapi/front_office.yaml#/components/schemas/FilterCompare/additionalProperties/properties",
        "web-api/openapi/front_office.yaml#/components/schemas/FilterCombine/properties",
        "web-api/openapi/front_office.yaml#/components/schemas/FilterIds/properties",
      ],
      rules: {
        "ibm-property-casing-convention": "off",
      },
    },
    {
      files: [
        "web-api/openapi/front_office.yaml#/components/schemas/FilterCombine/properties",
      ],
      rules: {
        "ibm-no-circular-refs": "off",
      },
    },
  ],
};
