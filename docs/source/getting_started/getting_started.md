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

For example the ingestion of (the first few paragraphs of the public domain book DRACULA by Bram Stoker):

```bash
curl -X POST "$URL/documents" \
    --header "authorizationToken: $BACKOFFICE_TOKEN" \
    --header "Content-Type: application/json" \
    --data '{
        "documents": [
            {
                "id": "xayn_efd5604c",
                "snippet": "3 May. Bistritz.—Left Munich at 8:35 P. M., on 1st May, arriving at Vienna early next morning; should have arrived at 6:46, but train was an hour late. Buda-Pesth seems a wonderful place, from the glimpse which I got of it from the train and the little I could walk through the streets. I feared to go very far from the station, as we had arrived late and would start as near the correct time as possible. The impression I had was that we were leaving the West and entering the East; the most western of splendid bridges over the Danube, which is here of noble width and depth, took us among the traditions of Turkish rule.\n\nWe left in pretty good time, and came after nightfall to Klausenburgh. Here I stopped for the night at the Hotel Royale. I had for dinner, or rather supper, a chicken done up some way with red pepper, which was very good but thirsty. (Mem., get recipe for Mina.) I asked the waiter, and he said it was called “paprika hendl,” and that, as it was a national dish, I should be able to get it anywhere along the Carpathians. I found my smattering of German very useful here; indeed, I don’t know how I should be able to get on without it.\n\nHaving had some time at my disposal when in London, I had visited the British Museum, and made search among the books and maps in the library regarding Transylvania; it had struck me that some foreknowledge of the country could hardly fail to have some importance in dealing with a nobleman of that country. I find that the district he named is in the extreme east of the country, just on the borders of three states, Transylvania, Moldavia and Bukovina, in the midst of the Carpathian mountains; one of the wildest and least known portions of Europe. I was not able to light on any map or work giving the exact locality of the Castle Dracula, as there are no maps of this country as yet to compare with our own Ordnance Survey maps; but I found that Bistritz, the post town named by Count Dracula, is a fairly well-known place. I shall enter here some of my notes, as they may refresh my memory when I talk over my travels with Mina.\n\nIn the population of Transylvania there are four distinct nationalities: Saxons in the South, and mixed with them the Wallachs, who are the descendants of the Dacians; Magyars in the West, and Szekelys in the East and North. I am going among the latter, who claim to be descended from Attila and the Huns. This may be so, for when the Magyars conquered the country in the eleventh century they found the Huns settled in it. I read that every known superstition in the world is gathered into the horseshoe of the Carpathians, as if it were the centre of some sort of imaginative whirlpool; if so my stay may be very interesting. (Mem., I must ask the Count all about them.)\n\nI did not sleep well, though my bed was comfortable enough, for I had all sorts of queer dreams. There was a dog howling all night under my window, which may have had something to do with it; or it may have been the paprika, for I had to drink up all the water in my carafe, and was still thirsty. Towards morning I slept and was wakened by the continuous knocking at my door, so I guess I must have been sleeping soundly then. I had for breakfast more paprika, and a sort of porridge of maize flour which they said was “mamaliga,” and egg-plant stuffed with forcemeat, a very excellent dish, which they call “impletata.” (Mem., get recipe for this also.) I had to hurry breakfast, for the train started a little before eight, or rather it ought to have done so, for after rushing to the station at 7:30 I had to sit in the carriage for more than an hour before we began to move. It seems to me that the further east you go the more unpunctual are the trains. What ought they to be in China?\n\nAll day long we seemed to dawdle through a country which was full of beauty of every kind. Sometimes we saw little towns or castles on the top of steep hills such as we see in old missals; sometimes we ran by rivers and streams which seemed from the wide stony margin on each side of them to be subject to great floods. It takes a lot of water, and running strong, to sweep the outside edge of a river clear. At every station there were groups of people, sometimes crowds, and in all sorts of attire. Some of them were just like the peasants at home or those I saw coming through France and Germany, with short jackets and round hats and home-made trousers; but others were very picturesque. The women looked pretty, except when you got near them, but they were very clumsy about the waist. They had all full white sleeves of some kind or other, and most of them had big belts with a lot of strips of something fluttering from them like the dresses in a ballet, but of course there were petticoats under them. The strangest figures we saw were the Slovaks, who were more barbarian than the rest, with their big cow-boy hats, great baggy dirty-white trousers, white linen shirts, and enormous heavy leather belts, nearly a foot wide, all studded over with brass nails. They wore high boots, with their trousers tucked into them, and had long black hair and heavy black moustaches. They are very picturesque, but do not look prepossessing. On the stage they would be set down at once as some old Oriental band of brigands. They are, however, I am told, very harmless and rather wanting in natural self-assertion.",
                "split": true,
            }
        ]
    }'

```

could split the text into three part:

- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 802 }` containing `"3 May.\n\nBistritz.—Left Munich at 8:35 P. M., on 1st May, arriving at Vienna early next morning; should have arrived at 6:46, but train was an hour late.\n\nBuda-Pesth seems a wonderful place, from the glimpse which I got of it from the train and the little I could walk through the streets.\n\nI feared to go very far from the station, as we had arrived late and would start as near the correct time as possible.\n\nThe impression I had was that we were leaving the West and entering the East; the most western of splendid bridges over the Danube, which is here of noble width and depth, took us among the traditions of Turkish rule.\n\nWe left in pretty good time, and came after nightfall to Klausenburgh.\n\nHere I stopped for the night at the Hotel Royale.\n\nI had for dinner, or rather supper, a chicken done up some way with red pepper, which was very good but thirsty.\n\n(Mem., get recipe for Mina.)\n\nI asked the waiter, and he said it was called “paprika hendl,” and that, as it was a national dish, I should be able to get it anywhere along the Carpathians.\n\nI found my smattering of German very useful here; indeed, I don’t know how I should be able to get on without it.\n\nHaving had some time at my disposal when in London, I had visited the British Museum, and made search among the books and maps in the library regarding Transylvania; it had struck me that some foreknowledge of the country could hardly fail to have some importance in dealing with a nobleman of that country.\n\nI find that the district he named is in the extreme east of the country, just on the borders of three states, Transylvania, Moldavia and Bukovina, in the midst of the Carpathian mountains; one of the wildest and least known portions of Europe."`
- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 4 }` containing `"I was not able to light on any map or work giving the exact locality of the Castle Dracula, as there are no maps of this country as yet to compare with our own Ordnance Survey maps; but I found that Bistritz, the post town named by Count Dracula, is a fairly well-known place.\n\nI shall enter here some of my notes, as they may refresh my memory when I talk over my travels with Mina.\n\nIn the population of Transylvania there are four distinct nationalities: Saxons in the South, and mixed with them the Wallachs, who are the descendants of the Dacians; Magyars in the West, and Szekelys in the East and North.\n\nI am going among the latter, who claim to be descended from Attila and the Huns.\n\nThis may be so, for when the Magyars conquered the country in the eleventh century they found the Huns settled in it.\n\nI read that every known superstition in the world is gathered into the horseshoe of the Carpathians, as if it were the centre of some sort of imaginative whirlpool; if so my stay may be very interesting.\n\n(Mem., I must ask the Count all about them.)\n\nI did not sleep well, though my bed was comfortable enough, for I had all sorts of queer dreams.\n\nThere was a dog howling all night under my window, which may have had something to do with it; or it may have been the paprika, for I had to drink up all the water in my carafe, and was still thirsty.\n\nTowards morning I slept and was wakened by the continuous knocking at my door, so I guess I must have been sleeping soundly then.\n\nI had for breakfast more paprika, and a sort of porridge of maize flour which they said was “mamaliga,” and egg-plant stuffed with forcemeat, a very excellent dish, which they call “impletata.” (Mem., get recipe for this also.)"`
- `"snippet_id": { document_id: "xayn_efd5604c",  sub_id: 300 }` containing `"I had to hurry breakfast, for the train started a little before eight, or rather it ought to have done so, for after rushing to the station at 7:30 I had to sit in the carriage for more than an hour before we began to move.\n\nIt seems to me that the further east you go the more unpunctual are the trains.\n\nWhat ought they to be in China?\n\nAll day long we seemed to dawdle through a country which was full of beauty of every kind.\n\nSometimes we saw little towns or castles on the top of steep hills such as we see in old missals; sometimes we ran by rivers and streams which seemed from the wide stony margin on each side of them to be subject to great floods.\n\nIt takes a lot of water, and running strong, to sweep the outside edge of a river clear.\n\nAt every station there were groups of people, sometimes crowds, and in all sorts of attire.\n\nSome of them were just like the peasants at home or those I saw coming through France and Germany, with short jackets and round hats and home-made trousers; but others were very picturesque.\n\nThe women looked pretty, except when you got near them, but they were very clumsy about the waist.\n\nThey had all full white sleeves of some kind or other, and most of them had big belts with a lot of strips of something fluttering from them like the dresses in a ballet, but of course there were petticoats under them.\n\nThe strangest figures we saw were the Slovaks, who were more barbarian than the rest, with their big cow-boy hats, great baggy dirty-white trousers, white linen shirts, and enormous heavy leather belts, nearly a foot wide, all studded over with brass nails.\n\nThey wore high boots, with their trousers tucked into them, and had long black hair and heavy black moustaches.\n\nThey are very picturesque, but do not look prepossessing.\n\nOn the stage they would be set down at once as some old Oriental band of brigands.\n\nThey are, however, I am told, very harmless and rather wanting in natural self-assertion."`

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
