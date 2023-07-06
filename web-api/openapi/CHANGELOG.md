# 2.4.0 - 2023-06-29

- Add property filters to the `/users/{id}/personalized_documents` and `/semantic_search` endpoints:
    - string equality
    - array of strings containment
    - logical combinators

# 2.3.0 - 2023-06-28

- added endpoints for retrieving and extending the indexed property schema

# 2.2.0 - 2023-06-26

- Allow empty string for properties
- Do not trim string properties

# 2.1.0 - 2023-06-22

- Allow empty array and null value for properties

# 2.0.0 - 2023-06-19

- trim whitespace around ids, snippets, tags and string properties
- disallow underscore prefixes in all ids and dots in property ids
- limit properties to boolean, number, string or array of strings and a total size of 2.5KB
- limit snippets and string properties to a total size of 2KB each
- remove the `min_similarity` request option from the `/semantic_search` endpoint
- move the candidates endpoint to `/documents/_candidates` and deprecate the old endpoints
- add `include_properties` request option to the `/users/{id}/personalized_documents` and `/semantic_search` endpoints and include the document properties in the response accordingly
