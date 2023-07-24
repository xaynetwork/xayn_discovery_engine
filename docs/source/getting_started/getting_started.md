# Overview

Here we take a high-level look at how the system works. The API is divided into two parts: the back office and the front office.

The back office system can be used to ingest your content into the system. Every piece of content you ingest is managed as one document within the Xayn system.

During ingestion, the system creates a mathematical representation of the document which is used to match the document to the user's interests and searches.

Once we have the documents in the system, we can use the front office to implement different use cases. For example, to have a ‘for you’ section, we need to add user interactions (clicks, reading, viewing) with documents. With each interaction, the system creates or updates a model that represents the user’s interests each time we add an interaction. Each user has a unique model that is used to return individually personalised search results and recommendations, which we are called personalised documents within the Xayn system.

Later, we will discuss other ways to get personalised documents without adding interactions.
With the front office, it is also possible to implement other use cases such as 'more like this', semantic and hybrid search.

# Getting started

To use the service, we first need to set up the authentication headers.  
We have two authentication tokens, one to connect to the back office and one to connect to the front office.  
To authenticate, we need to set the `authenticationToken` header to one of them, depending on what we need to do.
As our API expects all request bodies to be JSON encoded, we also need to set the `Content-Type` header to `application/json`.

# Ingest

We can use the back office endpoint [`/documents`](https:/docs.xayn.com/back_office.html#operation/createDocuments) to ingest documents.

We will ingest a document that represents this article: [https://xayn.com/blog/the-initial-challenge](https://xayn.com/blog/the-initial-challenge).

```bash
curl -X POST https://<url>/documents
    -H "authorizationToken: <back_office_token>"
    -H "Content-Type: application/json"
    -d '{
        "documents": [
            {
                "id": "xayn_cd5604c",
                "snippet": "The voices that are demanding better privacy protection and ownership of our own data are increasingly louder, there's a backlash towards these practices. At Xayn, our mission is to provide personalisation without user data leaving the device, maintaining absolute privacy. We use semantic similarity and centers of interest to understand user preferences and present better matching articles. With our model Xaynia, we offer semantic similarity and search with minimal energy consumption and at a low price, making it highly energy-efficient compared to other transformer models.",
                summarize: false,

                "properties": {
                    "title": "The initial challange",
                    "link": "https://xayn.com/blog/the-initial-challenge",
                    "image": "https://uploads-ssl.webflow.com/5ef08ebd35ddb63551189655/641320bc6be72c5453f4d98d_Blog%20Posts%20Visuals%20-%2003%20Mar%202023-p-2600.png",
                    "location" : ["germany", "berlin", "conference"],
                }
            }
        ]
    }'
```

The endpoint takes a list of documents to ingest.

Each document has a unique identifier that can be used to refer to it in the system.

The `snippet` field is used to inform the system about the content of the document; it is used as input to Xaynia to generate a mathematical representation of the document that we can use to match similar documents.

For this reason, it is essential that the snippet clearly represents the content of the document. In this case, we took a few representative sentences from the document and used them as a snippet. Since the amount of data that Xaynia can analyse is limited, if it is not possible to provide a concise snippet, we can use the per-document option `summarize`; when enabled, the system will summarise the content of the document to create a snippet.

The `properties` field is completely optional. It can contain custom data that can be used for filtering and that the system will return when a document is part of the result of a query.

The data that can be included in the properties is limited in terms of type and size. We support numbers, strings, boolean, date and list of strings, none of which are nullable. Please see [createDocuments](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) for more information on properties.

This example assumes that we will eventually display the returned documents as a 'for-you' section, where we want to display an article's image, title, text preview, and a link (for click-through), so we have included these specific properties during ingestion.

# Recommendations: Personalised documents

After ingestion, we can use the front office to retrieve recommendations, which we call personalised documents, and implement a 'for you' section.

From a system perspective, a user is represented by an ID that is needed to group their interactions. We don't need to know who this user is, so it is preferable to create this ID in a privacy-protecting way. For example, create a hash method that converts your user into an ID hash. Please ensure you don't use any sensitive or personally identifiable information (PII).

Let's use `u1234` as the user ID for our example.

We ask the system for [personalised documents](https://docs.xayn.com/front_office.html#tag/search/operation/getPersonalizedDocuments) for this user.

```bash
curl https://<url>/users/u1234/personalized_documents
    -H "authorizationToken: <front_office_token>"
```

As we can see, this returns with `409` status code and the following body:

```json
{ "kind": "NotEnoughInteractions" }
```

When there is an error, the system uses the 'kind' field to specify what kind of error has occurred. There may also be a `details` field.

In this case, we have 'NotEnoughInteractions'. This means that the system needs to receive more interactions from the user to learn their interests and cannot provide personalised documents at this time.

We can add an [interaction](https://docs.xayn.com/front_office.html#tag/interaction) between our user `u1234` and the document `xayn_cd5604c`:

```bash
curl -X POST https://<url>/users/u1234/interactions
    -H "authorizationToken: <front_office_token>"
    -H "Content-Type: application/json"
    -d '{
        "documents": [
           { "id": "xayn_cd5604c" }
        ]
    }'

```

```{note}
Please note that if an interaction between a user and a document is added, the document will **not** be part of the documents returned for future calls to the personalised endpoint.
```

Let's ask for personalised documents again now:

```bash
curl https://<url>/users/u1234/personalized_documents?include_properties=true
    -H "authorizationToken: <front_office_token>"
```

As a result, we will get something like:

```json
{
  "documents": [
    {
      "id": "xayn_5283ef3",
      "score": 0.8736,
      "properties": {
        "title": "Why every bit matters",
        "link": "https://www.xayn.com/blog/why-every-bit-matters",
        "image": "https://uploads-ssl.webflow.com/5ef08ebd35ddb63551189655/61447d6ebda40f1487c6ed9a_noah-silliman-2ckQ4BrvpC4-unsplash-p-2000.jpeg"
      }
    },
    {}
  ]
}
```

In the request, we ask the system to include the properties of the returned documents. We can use this data to implement a 'more like this' section.

We also have a 'score' field which represents how well the documents match the user's interests. The higher the number, the better the documents match.

# Search

Depending on the use-case searching for documents can be achieved as a search for documents _similar_ to a given document or as a _free-text search_. Both variants can then be run as a anonymous search or a search that is personalized. Personalization comes in two fashions, with a _user-id_ or by providing a interaction _history_.

## Similar documents

In this search variant only a _document id_ must be provided to the [`/semantic_search`](https://docs.xayn.com/front_office.html#tag/front-office/operation/getSimilarDocuments) endpoint.

```bash
curl -X POST https://<url>/semantic_search
    -H "authorizationToken: <front_office_token>"
    -H "Content-Type: application/json"
    -d '{
        "document": { "id": "xayn_cd5604c" }
    }'
```

The result contains a list of documents that are similar to the provided document (identified by its _document id_).

## Free Text search

Just like [Similar documents](#similar-documents) it is also possible to run a free text search.

```bash
curl -X POST https://<url>/semantic_search
    -H "authorizationToken: <front_office_token>"
    -H "Content-Type: application/json"
    -d '{
        "document": { "query": "Privacy and security" }
    }'
```

The quality of the results can vary on the length of the provided query. Short queries usually yield better results with the [hybrid search option](https://docs.xayn.com/front_office.html#tag/front-office/operation/getSimilarDocuments) enabled, that combines semantic and bm25 search:

```json
{
  "document": { "query": "Privacy and security" },
  "enable_hybrid_search": true
}
```

## Personalised search

To personalise search results for a specific user, any search can also be combined with an `user id` or a user `history`, which is a list of interactions of a particular user. The option to use a user history of interactions instead of a user id enables a personalised search without the need for Xayn to store a user id or history of interactions.

This is how we ask the system for a personalised search result for a [user](#recommendations-personalised-documents):

```bash
curl -X POST https://<url>/semantic_search
    -H "authorizationToken: <front_office_token>"
    -H "Content-Type: application/json"
    -d '{
        "document": { "query": "Privacy and security" },
        "personalize": {
            "exclude_seen": true,
            "user": {
                "id": "u1234",
            }
        }
    }'
```

The result is a list of documents that match the search query, which are additionally sorted by relevance to the user's interests based on their content interactions.

Alternatively a history of interactions can be used instead of a user id to ask for personalised documents:

```json
"personalize": {
    "exclude_seen": true,
    "user": {
       "history": [
            {
                "id": "valid_doc_id1",
                "timestamp": "2000-05-14T20:22:50Z"
            },
            {
                "id": "valid_doc_id2",
                "timestamp": "2000-05-15T20:22:50Z"
            }
        ]
    }
}
```

```{note}
Please note: `"exclude_seen": true` (default true) filters out documents from search results, that were interacted with by the user or have been provided in the history.
```

# Filters

Finding specific documents in large datasets based on a key-phrase or their relation to other documents can often be challenging. To address this issue, we can employ a structured filter on one or more of the `properties` fields to narrow down the search scope.

The `filter` functionality is available in the [`/semantic_search`](https://docs.xayn.com/front_office.html#tag/search/operation/getSimilarDocuments) and [`/users/{user_id}/personalized_documents`](https://docs.xayn.com/front_office.html#tag/search/operation/getPersonalizedDocuments) endpoints, and it involves a two-step process:

1. Indexing the desired property field for the filter to operate on.
2. Applying the `filter` in the request to `/semantic_search` or `/users/{user_id}/personalized_documents`.

```{note}
Please note that the __first step__ is necessary to leverage the filtering at all.
```

## Indexing a filter property

First lets check which properties are already indexed: 

```bash
curl --location 'https://<url>/documents/_indexed_properties' \
--header 'authorizationToken: <back_office_token>>'
```

This returns just the `publication_date`, which is indexed by default.

```json
{
    "properties": {
        "publication_date": {
            "type": "date"
        }
    }
}
```

Next, we can proceed to include our desired property, specifically the `tags` field, in the index. To accomplish this, we need to provide the name and type of the property. The available types for indexing are [`keyword, keyword[], boolean, date, number`](https://docs.xayn.com/back_office.html#tag/property-indexing/operation/createIndexedProperties).

```bash
curl --location 'https://<url>/documents/_indexed_properties' \
--header 'Content-Type: application/json' \
--header 'authorizationToken: <back_office_token>' \
--data '{
    "properties": {
        "location": {
            "type": "keyword[]"
        }
    }
}'

```

After a short indexing period, depending on the number of ingested documents, we can apply filters to our requests.

## Applying a Filter

Applying a filter then just requires to use the `filter` property in the `/semantic_search` or `/users/{user_id}/personalized_documents` query parameter. In the following two examples we simply filter for the tag `conference`.

```{code-block} bash
:caption: /semantic_search

curl --location 'https://<url>/semantic_search' \
--header 'Content-Type: application/json' \
--header 'authorizationToken: <front_office_token>' \
--data '{
    "filter": {
        "location": {
            "$in": [
                "conference"
            ]
        }
    },
    "document": {
        "query": "Privacy and security"
    }
}'
```

In `personalized_documents` the filter is applied in a similar way: 

```{code-block} bash
:caption: /users/{user_id}/personalized_documents

curl --location 'https://<url>/users/<user_id>/personalized_documents' \
--header 'Content-Type: application/json' \
--header 'authorizationToken: <front_office_token>'
--data '{ "filter": {
    "location": {
        "$in": [
            "conference"
        ]
    }
}}'
```

# Candidates

The [`/candidates`](https://docs.xayn.com/back_office.html#tag/candidates) api is a set back-office requests that allows to globally define the documents that all apis can recommend or generate search results from.  Documents that are not part of the candidates set will not be included in search results or recommendations, but interactions with these documents are still stored and can still be recorded.

After ingesting documents we can check the candidates:

```bash
curl --location 'https://<url>/documents/candidates' \
--header 'authorizationToken: <back_office_token>'
```

This returns a list with all documents ids. By default all newly ingested documents are set to be candidates. This behavior can be changed by passing [`is_candidate`](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) or [`default_is_candidate`](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) in the ingestion request.

Then we can __change__ the candidates by sending a list of document-ids to the `candidates` endpoint:

```bash
curl --location --request PUT 'https://164pz0ca24.execute-api.eu-central-1.amazonaws.com/default/documents/candidates' \
--header 'Content-Type: application/json' \
--header 'authorizationToken: R8yTEtBCuIYqxLXFVnbZ2HVj1DH1fNx5BQwQpih7' \
--data '{
    "documents" :[{ "id": "u1234}, { "id": "u1232}, { "id": "u1231}]
}'
```

```{note}
Please note, that setting candidates can only be undone by sending the complete list of all ingested document-ids again. 
```

The candidates can facilitate fast transitions between different sets of documents without compromising the users' centers of interest (COIs) with which they were engaging. One practical scenario is handling outdated news articles that should not reappear in the recommendations. However, the past user interactions with those outdated articles should still influence the suggested documents.
