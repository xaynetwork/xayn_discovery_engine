# Ingest One Document
POST {{url}}/documents
authorizationToken: {{bo_token}}
{
  "documents": [
    {
      "id": "8d53f7ae-b2a0-4821-94d2-b9904b276166",
      "snippet": "Trailer ZDF-Sommerinterview Schwesig",
      "tags": [],
      "properties": {
        "title": "Trailer ZDF-Sommerinterview Schwesig",
        "body": "Das ZDF-Sommerinterview mit der kommissarischen SPD-Vorsitzenden Manuela Schwesig sehen Sie am Sonntag, 11. August um 19:10 Uhr im ZDF und im Livestream auf heute.de.",
        "link": "https://www.zdf.de/trailer-sommerinterview-schwesig-100.html",
        "image": "https://www.zdf.de/assets/manuela-schwesig-120~1280x720?cb=1559587803626",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    }
  ]
}

HTTP 201

# Set candidate

PUT {{url}}/candidates
authorizationToken: {{bo_token}}
{
    "documents" :[{ "id": "8d53f7ae-b2a0-4821-94d2-b9904b276166"}]
}

HTTP 204

# Get candidate

GET {{url}}/candidates
authorizationToken: {{bo_token}}

HTTP 200

[Asserts]
jsonpath "$.documents" count == 1

# Set ingest documents

POST {{url}}/documents
authorizationToken: {{bo_token}}
{
  "documents": [
    {
      "id": "55b83ea2-d136-456d-b31a-cbf9773382c8",
      "snippet": "Schüler im Hamsterrad",
      "tags": [],
      "properties": {
        "title": "Schüler im Hamsterrad",
        "body": "Von klein an ist der Wettbewerb in allen Lebenslagen allgegenwärtig. Schon Grundschüler sind gefangen im Hamsterrad. Schon Siebenjährige pauken zusätzlich noch in speziellen Instituten bis spät in die Nacht.",
        "link": "https://www.zdf.de/webstory-suedkorea-paukschule-100.html",
        "image": "https://www.zdf.de/assets/webdoku-suedkorea-paukschule1-100~1280x720?cb=1518091952269",
        "datePublished": "2022-04-23 22:00:00",
        "publication_date": "2021-01-01T02:07:14Z",
        "category": ""
      }
    },
    {
      "id": "8d53f7ae-b2a0-4821-94d2-b9904b276166",
      "snippet": "Trailer ZDF-Sommerinterview Schwesig",
      "tags": [],
      "properties": {
        "title": "Trailer ZDF-Sommerinterview Schwesig",
        "body": "Das ZDF-Sommerinterview mit der kommissarischen SPD-Vorsitzenden Manuela Schwesig sehen Sie am Sonntag, 11. August um 19:10 Uhr im ZDF und im Livestream auf heute.de.",
        "link": "https://www.zdf.de/trailer-sommerinterview-schwesig-100.html",
        "image": "https://www.zdf.de/assets/manuela-schwesig-120~1280x720?cb=1559587803626",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    },
    {
      "id": "62bb8165-69e7-4a02-bf39-1c7a8b400917",
      "snippet": "Dänemark - Deutschland live",
      "tags": [],
      "properties": {
        "title": "Dänemark - Deutschland live",
        "body": "Das Testspiel der DFB-Elf in Dänemark mit Livestream, Liveticker und Live-Statistiken. Anstoß ist um 20.45 Uhr.",
        "link": "https://www.zdf.de/sport/ticker-daenemark-deutschland-100.html",
        "image": "https://www.zdf.de/assets/sportdaten-ticker-dfb-testspiele-100~1280x720?cb=1602097296961",
        "datePublished": "2022-04-23 22:00:00",
        "category": "Sport"
      }
    },
    {
      "id": "68713297-9388-4f9a-986b-f226ae9f99f1",
      "snippet": "\"so geht MEDIEN\" ",
      "tags": [],
      "properties": {
        "title": "\"so geht MEDIEN\" ",
        "body": "Angebot von ARD, ZDF und Deutschlandradio startet",
        "link": "https://www.zdf.de/zdfunternehmen/engagement-112.html",
        "image": "https://www.zdf.de/assets/engagement-110~1280x720?cb=1478514833423",
        "datePublished": "2022-04-23 22:00:00",
        "category": "Das Unternehmen"
      }
    },
    {
      "id": "67670c11-9a9a-4a98-b031-b1101e465247",
      "snippet": "Morgenmagazin vom 15. September 2015",
      "tags": [],
      "properties": {
        "title": "Morgenmagazin vom 15. September 2015",
        "body": "Heute mit: Roundup Flüchtlinge, Politik und Flüchtlinge, Urteil zu Hartz IV, Automobilmarkt + China, Zwischenbilanz Mindestlohn, Service: Kinderbrillen, Gespräch Dieter Hecking, PK Loretta Lynch",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-15-september-2015-100.html",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    },
    {
      "id": "d99ec399-4200-4c54-83b1-ce2fd1869036",
      "snippet": "Morgenmagazin vom 28. September 2015",
      "tags": [],
      "properties": {
        "title": "Morgenmagazin vom 28. September 2015",
        "body": "Heute mit: Syrien-Strategie, Unbegleitete Flüchtlinge, Massenschlägerei unter Flüchtlingen, Regionalwahl Katalonien, VW, Mondfinsternis, Service: Test Grüner Tee, Volleyball-EM: Deutschland - Rumänien",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-28-september-2015-100.html",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    },
    {
      "id": "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee",
      "snippet": "Morgenmagazin vom 27. November 2015",
      "tags": [],
      "properties": {
        "title": "Morgenmagazin vom 27. November 2015",
        "body": "Heute mit: Deutsche Unterstützung Syrien, Trauerfeier Paris, Seawatch vor Lesbos, Sorgerechststreit Nicaragua, Vor AfD-Parteitag, moma-Adventskalender, Crüwells Woche, Fußball-EL: Schalke - Nikosia",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-27-november-2015-100.html",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    },
    {
      "id": "b8e79039-9702-42c1-adfe-ea5302352b73",
      "snippet": "Morgenmagazin vom 29. September 2015",
      "tags": [],
      "properties": {
        "title": "Morgenmagazin vom 29. September 2015",
        "body": "Heute mit: Treffen Obama-Putin, Neue Asylgesetze, Wasser auf dem Mars, 25 Jahre Einheit, Sicherheit im Aufnahmelager,Neues aus Wolfsburg, Service: Folgen VW, Fb-CL: Barca vor Leverkusen, FIFA-Update",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-29-september-2015-100.html",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    },
    {
      "id": "8bce6d6d-7e95-49c6-921a-d935ee2774c3",
      "snippet": "Morgenmagazin vom 31. August 2015",
      "tags": [],
      "properties": {
        "title": "Morgenmagazin vom 31. August 2015",
        "body": "Heute mit: Politik und Flüchtlinge, Ungarischer Grenzzaun, Auf Arbeitssuche: Fische ausnehmen, Reiseportale, Service: Kontaktlinsen, Basketball: EM-Generalprobe gegen Frankreich, LA-WM: Bilanz Peking",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-31-august-2015-100.html",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "datePublished": "2022-04-23 22:00:00",
        "category": ""
      }
    }
  ]
}

HTTP 201

# check candidate

GET {{url}}/candidates
authorizationToken: {{bo_token}}

HTTP 200

[Asserts]
jsonpath "$.documents" count == 9

# delete one document

DELETE {{url}}/documents/8d53f7ae-b2a0-4821-94d2-b9904b276166
authorizationToken: {{bo_token}}

HTTP 204

# Add Document Properties

PUT {{url}}/documents/8bce6d6d-7e95-49c6-921a-d935ee2774c3/properties
authorizationToken: {{bo_token}}
{
    "properties": {
        "body": "Heute mit: Politik und Flüchtlinge, Ungarischer Grenzzaun, Auf Arbeitssuche: Fische ausnehmen, Reiseportale, Service: Kontaktlinsen, Basketball: EM-Generalprobe gegen Frankreich, LA-WM: Bilanz Peking",
        "datePublished": "2022-04-23 22:00:00",
        "image": "https://www.zdf.de/assets/zdf-logo-100~1280x720?cb=1503513994134",
        "title": "Morgenmagazin vom 31. August 2015",
        "link": "https://www.zdf.de/migration/migration/morgenmagazin-vom-31-august-2015-100.html",
        "category": "",
        "test": "HAHAH"
    }
}

HTTP 204

# Get Document Properties

GET {{url}}/documents/8bce6d6d-7e95-49c6-921a-d935ee2774c3/properties
authorizationToken: {{bo_token}}

HTTP 200

[Asserts]
jsonpath "$.properties.test" == "HAHAH"

# two interactions

PATCH {{url}}/users/simon/interactions
authorizationToken: {{fo_token}}
{
    "documents" : [
        {
            "id" : "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee"
        },
        {
            "id" : "d99ec399-4200-4c54-83b1-ce2fd1869036"
        }
    ]
}

HTTP 204

# Do one interaction that was already submitted

PATCH {{url}}/users/simon/interactions
authorizationToken: {{fo_token}}
{
    "documents" : [
        {
            "id" : "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee"
        }
    ]
}

HTTP 204

# One new interaction

PATCH {{url}}/users/238468732462346/interactions
authorizationToken: {{fo_token}}
{
    "documents" : [{"id" : "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee"}]
}

HTTP 204

# Personalized Documents per User

GET {{url}}/users/simon/personalized_documents
authorizationToken: {{fo_token}}

HTTP 200

[Asserts]
jsonpath "$.documents" count > 0

# Check Similar Documents

POST {{url}}/semantic_search
authorizationToken: {{fo_token}}
{
    "document" :{"id" : "8bce6d6d-7e95-49c6-921a-d935ee2774c3"}
}

HTTP 200

[Asserts]
jsonpath "$.documents[0].id" == "8bce6d6d-7e95-49c6-921a-d935ee2774c3"

# Personalized Documents for a given History

POST {{url}}/semantic_search
authorizationToken: {{fo_token}}
{
    "document" :{
        "id" : "67670c11-9a9a-4a98-b031-b1101e465247"
    },
    "personalize": {
        "user": {
            "history": [
                {
                    "id": "67670c11-9a9a-4a98-b031-b1101e465247"
                }
            ]
        }
    }
}

HTTP 200

[Asserts]
jsonpath "$.documents" count > 0

# Run Free Text Search

POST {{url}}/semantic_search
authorizationToken: {{fo_token}}
{
    "document" :{ "query": "A very violent Movie."}
}

HTTP 200

[Asserts]
jsonpath "$.documents[0].id" == "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee"

# Set candidate

PUT {{url}}/candidates
authorizationToken: {{bo_token}}
{
    "documents" :[{ "id": "55b83ea2-d136-456d-b31a-cbf9773382c8"}]
}

HTTP 204

# Personalized Documents for only one candidate

GET {{url}}/users/simon/personalized_documents
authorizationToken: {{fo_token}}

HTTP 200

[Asserts]
jsonpath "$.documents" count == 8

# Delete all documents

DELETE {{url}}/documents
authorizationToken: {{bo_token}}
[Options]
very-verbose: true

{
    "documents": [
        "55b83ea2-d136-456d-b31a-cbf9773382c8",
        "8d53f7ae-b2a0-4821-94d2-b9904b276166",
        "62bb8165-69e7-4a02-bf39-1c7a8b400917",
        "68713297-9388-4f9a-986b-f226ae9f99f1",
        "67670c11-9a9a-4a98-b031-b1101e465247",
        "d99ec399-4200-4c54-83b1-ce2fd1869036",
        "48aee97f-f9f3-4a2a-b4d7-78c1171de9ee",
        "b8e79039-9702-42c1-adfe-ea5302352b73",
        "8bce6d6d-7e95-49c6-921a-d935ee2774c3"
    ]
}

HTTP 204
