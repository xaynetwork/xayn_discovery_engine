# Overview

Here we take a high-level look at how the system works. The API is divided into two parts: the back office and the front office.

The back office system can be used to ingest your content into the system. Every piece of content you ingest is managed as one document within the Xayn system.

A document is comprised of one or more snippets. Each snippet represents a piece of the content of the document. In the simplest case, you only have one snippet per document in such a case you can mentally replace snippet with document in most of the remaining documentation. Through as there are limits on
how long a snippet can be, we provide functionality to either summarize the content or split the content of a document into multiple snippets. In many places both a document id and the id of a specific snippet can be used.

During ingestion, the system creates a mathematical representation of each snippet which is used to match the snippets to the user's interests and searches.

Once we have the documents in the system, we can use the front office to implement different use cases. For example, to have a ‘for you’ section, we need to add user interactions (clicks, reading, viewing) with documents or snippets. With each interaction, the system creates or updates a model that represents the user’s interests each time we add an interaction. Each user has a unique model that is used to return individually personalised search results and recommendations in form of a list of best matching snippets/documents.

Later, we will discuss other ways to get personalised documents without adding interactions.
With the front office, it is also possible to implement other use cases such as 'more like this', semantic and hybrid search.

# Getting started

To use the service, we first need to set up the authentication headers.
We have two authentication tokens, one to connect to the back office and one to connect to the front office.
To authenticate, we need to set the `authenticationToken` header to one of them, depending on what we need to do.
As our API expects all request bodies to be JSON encoded, we also need to set the `Content-Type` header to `application/json`.

In the following examples, we are going to use three environment variables: `$URL`, `$BACKOFFICE_TOKEN`, and `$FRONTOFFICE_TOKEN`.
To try the examples you need to set them to the values for your system beforehand:

```bash
export URL="<url>"
export BACKOFFICE_TOKEN="<backoffice_token>"
export FRONTOFFICE_TOKEN="<frontoffice_token>"
```

# Ingest

We can use the back office endpoint [`/documents`](https:/docs.xayn.com/back_office.html#operation/createDocuments) to ingest documents.

We will ingest a document that represents this article: [https://xayn.com/blog/the-initial-challenge](https://xayn.com/blog/the-initial-challenge).

```bash
curl -X POST "$URL/documents" \
    --header "authorizationToken: $BACKOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
            {
                "id": "xayn_cd5604c",
                "snippet": "The voices that are demanding better privacy protection and ownership of our own data are increasingly louder, there is a backlash towards these practices. At Xayn, our mission is to provide personalisation without user data leaving the device, maintaining absolute privacy. We use semantic similarity and centers of interest to understand user preferences and present better matching articles. With our model Xaynia, we offer semantic similarity and search with minimal energy consumption and at a low price, making it highly energy-efficient compared to other transformer models.",
                "summarize": false,
                "properties": {
                    "title": "The initial challange",
                    "link": "https://xayn.com/blog/the-initial-challenge",
                    "image": "https://uploads-ssl.webflow.com/5ef08ebd35ddb63551189655/641320bc6be72c5453f4d98d_Blog%20Posts%20Visuals%20-%2003%20Mar%202023-p-2600.png",
                    "location" : ["germany", "berlin", "conference"]
                },
            },
            {
                "id": "xayn_ff5604c",
                "snippet": "If you only ingested one short document you can't really try out any of the functional, so here is another document",
                "summarize": false,
            },
            {
                "id": "00000",
                "snippet": "There are just very few constraints on what an id can be, this means that most times you can just plug in ides from any other system you use to store documents in. But be aware tht ids are strings so 0, 00, and 000 are all different ids.",
                "summarize": false,
            },
            {
                "id": "xayn_604c",
                "snippet": "Privacy protection and ownership is a topic of another document, so is semantic search.",
                "summarize": false,
            }
        ]
    }'

```

The endpoint takes a list of documents to ingest.

Each document has a unique identifier that can be used to refer to it in the system.

The `snippet` field is used to inform the system about the content of the document; it is used as input to Xaynia to generate a mathematical representation of the document that we can use to match similar documents.

For this reason, it is essential that the snippet clearly represents the content of the document. In this case, we took a few representative sentences from the document and used them as a snippet. Since the amount of data that Xaynia can analyze is limited, we can use the per-document option `summarize` when it is not possible to provide a concise snippet; when enabled, the system will summarise the content of the document to create a snippet. Alternatively, the `split` option can be used which will split the document into multiple snippets,
this can be used to get better result on larger documents. More information on this can be found in the section  [split documents](#split-documents).

For example the ingestion of (from BAnz AT 13.07.2023 B1 page 3):

```bash
curl -X POST "$URL/documents" \
    --header "authorizationToken: $BACKOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
            {
                "id": "xayn_efd5604c",
                "snippet": "6.2  Die  Vergabe  von  Unteraufträgen  hat  nach  Möglichkeit  im  Wettbewerb  zu  erfolgen.  Bei  der  Einholung  von  An-\ngeboten  für  Unteraufträge  sind  kleine  und  mittlere,  nicht  konzerngebundene  Unternehmen  soweit  möglich  zu  betei-\nligen.  Die  in  Betracht  kommenden  Unternehmen  sind  dem  Auftraggeber  vom  Auftragnehmer  auf  Verlangen  vor  der\nErteilung des Unterauftrags zu benennen.\n6.3  Der Auftragnehmer zeigt dem Auftraggeber jeden Unterauftrag sowie jeden Wechsel eines Unterauftragnehmers\nnach Erteilung des jeweiligen Unterauftrags bis zum Ende der jeweiligen Vertragslaufzeit unverzüglich und unaufge-\nfordert in Textform an. Maßgeblich ist das Datum des Vertragsschlusses. Dabei teilt der Auftragnehmer  mindestens\nden Namen und die Anschrift des Unterauftragnehmers mit sowie den Gegenstand des Unterauftrags. Die Anzeige-\npflicht  entfällt,  wenn  dem  Auftraggeber  die  Informationen  bereits  aus  dem  Angebot  des  Auftragnehmers  bzw.  den\nVergabeunterlagen bekannt sind.\n6.4  Hat der Auftraggeber in der Bekanntmachung oder in den Vergabeunterlagen Anforderungen über die Eignung\noder Auftragserfüllung für Unterauftragnehmer aufgestellt, sind diese von allen Unterauftragnehmern zu erfüllen. Dies\ngilt auch im Fall des Austauschs von Unterauftragnehmern während der Vertragslaufzeit. Der Auftragnehmer legt dem\nAuftraggeber  erforderliche  Nachweise  seiner  Unterauftragnehmer  unverzüglich  und  unaufgefordert  mit  der  Anzeige\ngemäß Nummer 6.3 vor.\n6.5  Vergibt der Auftragnehmer Unteraufträge, so hat er durch entsprechende Vereinbarungen mit den Unterauftrag-\nnehmern  dem  Auftraggeber  die  gleichen  Rechte  und  Ansprüche  zu  verschaffen,  die  der  Auftraggeber  gegen  den\nAuftragnehmer hat. Hierzu gehören auch die Nutzungsrechte des Auftraggebers an allen vom Auftragnehmer geschul-\ndeten Vertragsergebnissen.\n6.6  Gelingt dies dem Auftragnehmer im Einzelfall nicht, so hat er den Auftraggeber darüber unverzüglich in Textform\nzu  unterrichten  und  ihm  auf  Verlangen  Gelegenheit  zu  geben,  an  den  weiteren  Verhandlungen  mit  dem  jeweiligen\nUnterauftragnehmer teilzunehmen und die Entscheidung des Auftraggebers abzuwarten.\n6.7  Akzeptiert der Unterauftragnehmer die Vereinbarung entsprechender Regelungen nach Abschluss der weiteren\nVerhandlungen  nicht,  hat  der  Auftragnehmer  dies  dem  Auftraggeber  in  Textform  anzuzeigen,  das  Verhandlungs-\nergebnis vorzulegen und die Entscheidung des Auftraggebers darüber, ob er seine Einwilligung zum Vertragsschluss\nerklärt, einzuholen. Entscheidet sich der Auftraggeber nicht binnen eines Monats nach Zugang der Anzeige, so ist der\nAuftragnehmer  berechtigt,  den  Unterauftrag  entsprechend  dem  vorgelegten  Verhandlungsergebnis  abzuschließen.\nErteilt der Auftraggeber seine ausdrückliche Einwilligung zum Vertragsschluss oder erfolgt der Vertragsschluss nach\nAblauf der Monatsfrist, bleibt die Haftung des Auftragnehmers für die vertragsgemäße Ausführung seiner Leistungen\ngegenüber dem Auftraggeber unberührt.",
                "split": true,
            }
        ]
    }'

```

could split the text into three part:

- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 802 }` containing `"6.2  Die  Vergabe  von  Unteraufträgen  hat  nach  Möglichkeit  im  Wettbewerb  zu  erfolgen.\n\nBei  der  Einholung  von  An-\ngeboten  für  Unteraufträge  sind  kleine  und  mittlere,  nicht  konzerngebundene  Unternehmen  soweit  möglich  zu  betei-\nligen.\n\nDie  in  Betracht  kommenden  Unternehmen  sind  dem  Auftraggeber  vom  Auftragnehmer  auf  Verlangen  vor  der\nErteilung des Unterauftrags zu benennen.\n\n6.3  Der Auftragnehmer zeigt dem Auftraggeber jeden Unterauftrag sowie jeden Wechsel eines Unterauftragnehmers\nnach Erteilung des jeweiligen Unterauftrags bis zum Ende der jeweiligen Vertragslaufzeit unverzüglich und unaufge-\nfordert in Textform an.\n\nMaßgeblich ist das Datum des Vertragsschlusses.\n\nDabei teilt der Auftragnehmer  mindestens\nden Namen und die Anschrift des Unterauftragnehmers mit sowie den Gegenstand des Unterauftrags.\n\nDie Anzeige-\npflicht  entfällt,  wenn  dem  Auftraggeber  die  Informationen  bereits  aus  dem  Angebot  des  Auftragnehmers  bzw.\n\nden\nVergabeunterlagen bekannt sind.\n\n6.4  Hat der Auftraggeber in der Bekanntmachung oder in den Vergabeunterlagen Anforderungen über die Eignung\noder Auftragserfüllung für Unterauftragnehmer aufgestellt, sind diese von allen Unterauftragnehmern zu erfüllen.\n\nDies\ngilt auch im Fall des Austauschs von Unterauftragnehmern während der Vertragslaufzeit."`
- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 4 }` containing `"Der Auftragnehmer legt dem\nAuftraggeber  erforderliche  Nachweise  seiner  Unterauftragnehmer  unverzüglich  und  unaufgefordert  mit  der  Anzeige\ngemäß Nummer 6.3 vor.\n\n6.5  Vergibt der Auftragnehmer Unteraufträge, so hat er durch entsprechende Vereinbarungen mit den Unterauftrag-\nnehmern  dem  Auftraggeber  die  gleichen  Rechte  und  Ansprüche  zu  verschaffen,  die  der  Auftraggeber  gegen  den\nAuftragnehmer hat.\n\nHierzu gehören auch die Nutzungsrechte des Auftraggebers an allen vom Auftragnehmer geschul-\ndeten Vertragsergebnissen.\n\n6.6  Gelingt dies dem Auftragnehmer im Einzelfall nicht, so hat er den Auftraggeber darüber unverzüglich in Textform\nzu  unterrichten  und  ihm  auf  Verlangen  Gelegenheit  zu  geben,  an  den  weiteren  Verhandlungen  mit  dem  jeweiligen\nUnterauftragnehmer teilzunehmen und die Entscheidung des Auftraggebers abzuwarten.\n\n6.7  Akzeptiert der Unterauftragnehmer die Vereinbarung entsprechender Regelungen nach Abschluss der weiteren\nVerhandlungen  nicht,  hat  der  Auftragnehmer  dies  dem  Auftraggeber  in  Textform  anzuzeigen,  das  Verhandlungs-\nergebnis vorzulegen und die Entscheidung des Auftraggebers darüber, ob er seine Einwilligung zum Vertragsschluss\nerklärt, einzuholen."`
- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 300 }` containing `"Entscheidet sich der Auftraggeber nicht binnen eines Monats nach Zugang der Anzeige, so ist der\nAuftragnehmer  berechtigt,  den  Unterauftrag  entsprechend  dem  vorgelegten  Verhandlungsergebnis  abzuschließen.\n\nErteilt der Auftraggeber seine ausdrückliche Einwilligung zum Vertragsschluss oder erfolgt der Vertragsschluss nach\nAblauf der Monatsfrist, bleibt die Haftung des Auftragnehmers für die vertragsgemäße Ausführung seiner Leistungen\ngegenüber dem Auftraggeber unberührt."`

Be aware that `sub_id` is **not** guaranteed to have any specific ordering or structure, even through in practice
it will most times look sequential incremental. Similar the natural language based splitting is not
guaranteed to be deterministic. Especially over time, as we continue to improve the algorithm, the splitting may change.

The `properties` field is completely optional. It can contain custom data that can be used for filtering and that the system will return when a document is part of the result of a query.

The data that can be included in the properties is limited in terms of type and size. We support numbers, strings, boolean, date and list of strings, none of which are nullable. Please see [createDocuments](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) for more information on properties.

This example assumes that we will eventually display the returned documents as a 'for-you' section, where we want to display an article's image, title, text preview, and a link (for click-through), so we have included these specific properties during ingestion.

## Split Documents

We provide a functionality to extract multiple snippets from the provided content of a document.

The system uses Natural language processing (NLP) algorithms to split the document into multiple parts.

This algorithm will be improved over time. This means a document ingested now and a equal document ingested
in the future might have different splits. Additionally, not all NLP splitting algorithms are deterministic so we can't guarantee fully deterministic behavior even if changes to the algorithm are ignored.

Currently, automatic splitting works only with one language set when the system is set up; by default, it is English. If you need another one, please contact us.
We are working to add support for multiple languages to our text-splitting algorithms.

# Recommendations: Personalised documents

After ingestion, we can use the front office to retrieve recommendations, which we call personalised documents, and implement a 'for you' section.

From a system perspective, a user is represented by an ID that is needed to group their interactions. We don't need to know who this user is, so it is preferable to create this ID in a privacy-protecting way. For example, create a hash method that converts your user into an ID hash. Please ensure you don't use any sensitive or personally identifiable information (PII).

Let's use `u1234` as the user ID for our example.

We ask the system for [personalised documents](https://docs.xayn.com/front_office.html#tag/search/operation/getPersonalizedDocuments) for this user.

```bash
curl -X POST "$URL/users/u1234/personalized_documents" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json"
```

As we can see, this returns with `409` status code and the following body:

```json
{ "kind": "NotEnoughInteractions" }
```

When there is an error, the system uses the 'kind' field to specify what kind of error has occurred. There may also be a `details` field.

In this case, we have 'NotEnoughInteractions'. This means that the system needs to receive more interactions from the user to learn their interests and cannot provide personalised documents at this time.

We can add an [interaction](https://docs.xayn.com/front_office.html#tag/interaction) between our user `u1234` and the document `xayn_cd5604c`:

```bash
curl -X PATCH "$URL/users/u1234/interactions" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
             { "id": { "document_id": "xayn_cd5604c", "sub_id": 2 } }
        ]
    }'
```

Instead of a specific snippet a document as a whole can be specified, too:

```bash
curl -X PATCH "$URL/users/u1234/interactions" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
             { "id": "xayn_cd5604c" }
        ]
    }'
```

```{note}
Please note that if an interaction between a user and a document is added, the document will **not** be part of the documents returned for future calls to the personalised endpoint. This includes all snippets associated to that document
```

Let's ask for personalised documents again now:

```bash
curl -X POST "$URL/users/u1234/personalized_documents" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "include_properties": true
    }'
```

As a result, we will get something like:

```json
{
  "documents": [
  {
      "snippet_id": { "document_id": "xayn_5283ef3", "sub_id": 0 },
      "score": 0.8736,
      "properties": {
          "title": "Why every bit matters",
          "link": "https://www.xayn.com/blog/why-every-bit-matters",
          "image": "https://uploads-ssl.webflow.com/5ef08ebd35ddb63551189655/61447d6ebda40f1487c6ed9a_noah-silliman-2ckQ4BrvpC4-unsplash-p-2000.jpeg"
      }
  },
  { ... },
    ...
  ]
}
```

In the request, we ask the system to include the properties of the returned documents. We can use this data to implement a 'more like this' section.

We also have a `score` field which represents how well the documents match the user's interests. The higher the number, the better the documents match. It should be noted that the scores only have meaning in relation to other
scores from the same requests.

The field `snippet_id` identifies a specific snippet. The `document_id` in the `snippet_id` is the id of the document associated with the snippet.

If you do not use ingestion options like `split` and in turn only have one snippet per document then you can always use `snippet_id.document_id` and ignore the rest. In many places both a the full snippet id object or a document id string can be used. If documents which have multiple snippets are ingested it's highly recommended to always use the full snippet id.

# Search

Depending on the use-case searching for documents can be achieved as a search for documents _similar_ to a given snippet/document or as a _free-text search_. Both variants can then be run as a anonymous search or a search that is personalized. Personalization comes in two fashions, with a _user-id_ or by providing a interaction _history_.

## Similar documents

In this search variant either a _document id_ or a _snippet id_ must be provided to the [`/semantic_search`](https://docs.xayn.com/front_office.html#tag/front-office/operation/getSimilarDocuments) endpoint.

```bash
curl -X POST "$URL/semantic_search" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "document": { "id": { "document_id": "xayn_cd5604c", "sub_id": 2 } },
        "include_properties": true
    }'
```

The result contains a list of snippets that are similar to the identified snippet.

If only documents with a single snippet are used you can provide only the document id
like this: `"document": { "id": "xayn_cd5604c" },`.

Be aware that at this moment, this is equivalent to doing a semantic search on the first snippet of a document.
In the future, this behaviour could change to better represent the intent to search for something similar to the whole document. If you want to explicitly search for the first snippet use a _snippet id_.

## Free Text search

Just like [Similar documents](#similar-documents) it is also possible to run a free text search.

```bash
curl -X POST "$URL/semantic_search" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "document": {
            "query": "Privacy and security"
        },
        "include_properties": true
    }'
```

The quality of the results can vary on the length of the provided query. Short queries usually yield better results with the [hybrid search option](https://docs.xayn.com/front_office.html#tag/front-office/operation/getSimilarDocuments) enabled, that combines semantic and lexical search:

```json
{
  "enable_hybrid_search": true,
  "document": {
      "query": "Privacy and security"
  },
  "include_properties": true
}
```

## Personalised search

To personalise search results for a specific user, any search can also be combined with an `user id` or a user `history`, which is a list of interactions of a particular user. The option to use a user history of interactions instead of a user id enables a personalised search without the need for Xayn to store a user id or history of interactions.

This is how we ask the system for a personalised search result for a [user](#recommendations-personalised-documents):

```bash
curl -X POST "$URL/semantic_search" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "document": { "query": "Privacy and security" },
        "personalize": {
            "exclude_seen": true,
            "user": {
                "id": "u1234"
            }
        },
        "include_properties": true
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
                 "id": { "document_id": "valid_doc_id1", "sub_id": 2 },
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
curl -X GET "$URL/documents/_indexed_properties" \
    --header "authorizationToken: $BACKOFFICE_TOKEN"
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
curl -X POST "$URL/documents/_indexed_properties" \
    --header "authorizationToken: $BACKOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
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

curl -X POST "$URL/semantic_search" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "filter": {
            "location": {
                "$in": [
                    "conference",
                    "hamburg"
                ]
            }
        },
        "document": {
            "query": "Privacy and security"
        },
        "include_properties": true
    }'
```

In `personalized_documents` the filter is applied in a similar way:

```{code-block} bash
:caption: /users/{user_id}/personalized_documents

curl -X POST "$URL/users/u1234/personalized_documents" \
    --header "authorizationToken: $FRONTOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "filter": {
            "location": {
                "$in": [
                    "conference"
                ]
            }
        },
        "include_properties": true
    }'
```

# Candidates

The [`/candidates`](https://docs.xayn.com/back_office.html#tag/candidates) api is a set back-office requests that allows to globally define the documents that all apis can recommend or generate search results from. Snippets from documents that are not part of the candidates set will not be included in search results or recommendations, but interactions with these documents are still stored and can still be recorded.

Be aware that the candidates API is based on whole documents, it is not possible to set specific snippets.

After ingesting documents we can check the candidates:

```bash
curl -X GET "$URL/documents/candidates" \
    --header "authorizationToken: $BACKOFFICE_TOKEN"
```

This returns a list with all documents ids. By default all newly ingested documents are set to be candidates. This behavior can be changed by passing [`is_candidate`](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) or [`default_is_candidate`](https://docs.xayn.com/back_office.html#tag/documents/operation/createDocuments) in the ingestion request.

Then we can __change__ the candidates by sending a list of document-ids to the `candidates` endpoint:

```bash
curl -X PUT "$URL/documents/candidates" \
    --header "authorizationToken: $BACKOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
            { "id": "xayn_cd5604c" },
            { "id": "xayn_5283ef3" },
            { "id": "xayn_97afa2a" }
        ]
    }'
```

```{note}
Please note, that setting candidates can only be undone by sending the complete list of all ingested document-ids again.
```

The candidates can facilitate fast transitions between different sets of documents without compromising the users' centers of interest (COIs) with which they were engaging. One practical scenario is handling outdated news articles that should not reappear in the recommendations. However, the past user interactions with those outdated articles should still influence the suggested documents.
