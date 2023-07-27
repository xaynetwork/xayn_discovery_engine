# 2.5.0 - 2023-07-31

- Generalize scores in search results

# 2.4.1 - 2023-07-31

- Clarify requirements for filters regarding the indexed document properties

# 2.4.0 - 2023-07-20

- Add property filters to the `/users/{id}/personalized_documents` and `/semantic_search` front-office endpoints:
    - boolean equality
    - number range

# 2.3.2 - 2023-07-20

- Align the semantic search query string pattern with its description

# 2.3.1 - 2023-07-20

- Deprecate `GET /users/{id}/personalized_documents` with query params in favor of `POST` with request body

# 2.3.0 - 2023-07-06

- Add back-office endpoints for retrieving and extending the indexed property schema
- Add property filters to the `/users/{id}/personalized_documents` and `/semantic_search` front-office endpoints:
    - string equality
    - array of strings containment
    - date range
    - logical combinators

# 2.2.0 - 2023-06-26

- Allow empty string for properties
- Do not trim string properties

# 2.1.0 - 2023-06-22

- Allow empty array and null value for properties

# 2.0.0 - 2023-06-19

- Trim whitespace around ids, snippets, tags and string properties
- Disallow underscore prefixes in all ids and dots in property ids
- Limit properties to boolean, number, string or array of strings and a total size of 2.5KB
- Limit snippets and string properties to a total size of 2KB each
- Remove the `min_similarity` request option from the `/semantic_search` endpoint
- Move the candidates endpoint to `/documents/_candidates` and deprecate the old endpoints
- Add `include_properties` request option to the `/users/{id}/personalized_documents` and `/semantic_search` endpoints and include the document properties in the response accordingly
