

## Overview 

Here we take a high-level look at how the system works.  

The back office system can be used to ingest documents into the system. 

Documents are the items that the service uses to provide its functionality. During ingestion, the system creates a mathematical representation of the document which is used to match the document to the user's interests and searches. 

  

Once we have the documents in the system, we can use the front office to implement different use cases. For example, to have a 'for you' section, we need to add user interactions with documents. With each interaction, the system creates or updates a model that represents the user's interests each time we add an interaction. Each user has a unique model that is used to return personalised documents. 

Later, we will discuss other ways to get personalised documents without adding interactions. 
With the front office, it is also possible to implement other use cases such as 'more like this', semantic and hybrid search. 

# Getting started 

To use the service, we first need to set up the authentication headers.  
We have two authentication tokens, one to connect to the back office and one to connect to the front office.  
To authenticate, we need to set the `authenticationToken` header to one of them, depending on what we need to do. 
As our API expects all request bodies to be JSON encoded, we also need to set the `Content-Type` header to `application/json`. 

## Ingest 

We can use the back office endpoint  [https://xaynetwork.github.io/xayn_discovery_engine/back_office.html#operation/createDocuments](`/documents`) to ingest documents. 

We will ingest a document that represents this article: [https://xayn.com/blog/the-initial-challenge] (https://xayn.com/blog/the-initial-challenge). 

```bash
curl -X POST https://<url>/documents 
    -H "authorizationToken: <back_office_token>" 
    -H "Content-Type: application/json" 
    -d '{ 
        "documents": [ 
            { 
                "id": "xayn_cd5604c", 
                "snippet": "The voices that are demanding better privacy protection and ownership of our own data are increasingly louder, there's a backlash towards these practices. At Xayn, our mission is to provide personalisation without user data leaving the device, maintaining absolute privacy. We use semantic similarity and centers of interest to understand user preferences and present better matching articles. With our model Xaynia, we offer semantic similarity and search with minimal energy consumption and at a low price, making it highly energy-efficient compared to other transformer models.", 

                "properties": { 
                    "title": "The initial challange", 
                    "link": "https://xayn.com/blog/the-initial-challenge", 
                    "image": "https://uploads-ssl.webflow.com/5ef08ebd35ddb63551189655/641320bc6be72c5453f4d98d_Blog%20Posts%20Visuals%20-%2003%20Mar%202023-p-2600.png" 
                } 
            }
        ] 
    }' 
```  

The endpoint takes a list of documents to ingest. 

Each document has a unique identifier that can be used to refer to it in the system. 

  

The field 'snippet' field is used to inform the system about the content of the document; it is  used as input to Xaynia to generate a mathematical representation of the model that we can use to match similar documents. 

For this reason, it is essential that the snippet clearly represents the content of the article. In this case, we took a few representative sentences from the article and used them as a snippet. Since the amount of data that Xaynia can analyse is limited, if it is not possible to provide a concise snippet, we can use the per document option 'summarise' option per document; when enabled, the system will try to summarise the content of the snippet and use it as the input for the model. 

The 'properties' field is completely optional. It can contain custom data that can be used for filtering and that the system will return when a document is part of the result of a query. 

The data that can be included in the properties is limited in terms of type and size. We support numbers, strings, boolean, date and list of strings, none of which are nullable. Please see <link> for more information on properties. 

This example assumes that we will eventually display the returned documents as a 'for-you' section, where we want to display an article's image, title, text preview, and a link (for click-through), so we have included these specific properties during ingestion. 

## Personalised documents 

After ingestion, we can use the front office to retrieve personalised documents for our users and implement a 'for you' section. 

From a system perspective, a user is represented by an ID that is needed to group their interactions. We don't need to know who this user is, so it is preferable to create this ID in a privacy-protecting way. For example, create a hash method that converts your user into an ID hash. Ensure you don't use any sensitive or personally identifiable information (PII). 

Let's use `u1234` as the user ID for our example. 

We ask the system for personalised documents for this user. 

  

```bash
curl https://<url>/users/u1234/personalized_documents 
    -H "authorizationToken: <front_office_token>" 
```

As we can see, this returns with `409` status code and the following body: 

```json
{"kind":"NotEnoughInteractions"} 
``` 

When there is an error, the system uses the 'kind' field to specify what kind of error has occurred. There may also be a `details` field. 

In this case, we have 'NotEnoughInteractions'. This means that the system needs to receive more interactions from the user to learn their interests and cannot provide personalised documents at this time. 

We can add an interaction between our user `u1234` and the document `xayn_cd5604c`: 

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

Note that if an interaction between a user and a document is added, the document will **not** be part of the documents returned for future calls to the personalised endpoint. 

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
        { 
        }
    ]
} 
``` 

In the request, we ask the system to include the properties of the returned documents. We can use this data to implement a 'more like this' section. 

  

We also have a 'score' field which represents how well the documents match the user's interests. The higher the number, the better the documents match. 

  

## Search 

### Similar documents 

More like this 

  

### Text search 

text search and hybrid search 

  

## Personalised search 

with id or history 

  

## Candidate api 

some documents can be used only to learn user interests, and they will not be returned to the user 
