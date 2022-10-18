# How to set up Document and User APIs.

Just follow the steps described below. You need have `Rust`, `just` and `docker` with `docker compose` installed on your machine.

1. Build the Document API (ingestion), start the services (PostgreSQL and Elastic Search) and run the Document API.

```sh
just web-ingestion-up
```

2. Verify if Elastic Search service is "up", by sending a `GET http://localhost:9200` request. It should return `200` with some body.

```sh
curl -X GET 'http://localhost:9200'
```

3. Create an index in Elastic Search with mapping. Replace `index_name` with a desired name for an index.

```sh
curl -X PUT 'http://localhost:9200/index_name?pretty' \
  -H 'Content-Type: application/json' \
  -d '{
  "mappings": {
    "properties": {
      "snippet": {
        "type": "text"
      },
      "published_date": {
        "type": "date"
      },
      "embedding": {
        "type": "dense_vector",
        "dims": 128,
        "index": true,
        "similarity": "cosine"
      }
    }
  }
}'
```

4. Ingest some documents with calculated embeddings into Elastic Search.

```sh
curl -X POST 'http://localhost:3030/documents' \
  -H 'Content-Type: application/json' \
  -d '{
  "documents": [
    {
      "id": "document id 0001",
      "snippet": "Snippet of text that will be used to calculate embeddings.",
      "properties": {
        "title": "Document title"
      }
    }
  ]
}'
```

5. Verify you have ingested documents into Elastic Search, by opening Kibana on [http://localhost:6501](http://localhost:5601) or doing a search request.

```sh
curl -X GET 'http://localhost:9200/test_index/_search'
```

6. Start User API (personalization).

```sh
just web-api-up
```

7. Add some positive interactions to different documents. The system needs to have at least two centers of likes to be able to personalize documents.

```sh
curl -X PATCH 'http://localhost:3000/users/{user_id}/interactions' \
  -H 'Content-Type: application/json' \
  -D '{
  "documents": [
    { "id": "DOC_01", "type": "positive" },
    { "id": "DOC_01", "type": "positive" },
    { "id": "DOC_70", "type": "positive" }
  ]
}'
```

8. Fetch personalized documents.

```sh
curl -X GET 'http://localhost:3000/users/{user_id}/personalized_documents?count=100'
```
