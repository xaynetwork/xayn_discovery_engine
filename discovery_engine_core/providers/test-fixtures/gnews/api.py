#!/usr/bin/env python3

import requests
import json
import os

headers = {
    "Authorization": "Bearer " + os.environ["API_GATEWAY_TOKEN"],
}


url = "https://api-gw.xaynet.dev/news/v2/_sn"


queries = {
    "climate-change": {
        "q": '"Climate change"',
        "sortby": "publishedAt",
        "page_size": "10",
        "lang": "en",
        "countries": "AU",

    },
    "msft-vs-aapl": {
        "q": '("Bill Gates" OR "Tim Cook")',
        "sortby": "publishedAt",
        "page_size": "2",
        "lang": "de",
        "countries": "DE",
    },
}

for (key, params) in queries.items():
    response = requests.request("GET", url, headers=headers, params=params)
    data = json.loads(response.text)

    for article in data["articles"]:
        article["url"] = "https://example.com/path/to/article/"
        article["image"] = "https://uploads.example.com/image.png"
        article["source"]["name"] = "example.com"
        article["source"]["url"] = "https://example.com"

    with open(f"{key}.json", "w") as f:
        json.dump(data, f, indent=2)
