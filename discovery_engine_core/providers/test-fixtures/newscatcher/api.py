#!/usr/bin/env python3

import requests
import json
import os

headers = {
    "Authorization": "Bearer " + os.environ["API_GATEWAY_TOKEN"],
}

url = "https://api-gw.xaynet.dev/_sn"

queries = {
    "climate-change": {
        "q": '"Climate change"',
        "sort_by": "relevancy",
        "page_size": "2",
        "lang": "en",
        "countries": "AU",
    },
    "msft-vs-aapl": {
        "q": '("Bill Gates" || "Tim Cook")',
        "sort_by": "relevancy",
        "page_size": "2",
        "lang": "de",
        "countries": "DE",
    },
}

for (key, params) in queries.items():
    response = requests.request("GET", url, headers=headers, params=params)
    data = json.loads(response.text)

    for article in data["articles"]:
        article["author"] = "Anonymous"
        article["link"] = "https://example.com"
        article["clean_url"] = "example.com"
        article["rights"] = "example.com"
        article["authors"] = ["Anonymous"]
        article["media"] = "https://uploads.example.com/image.png"
        article["twitter_account"] = "@XaynHQ"

    with open(f"{key}.json", "w") as f:
        json.dump(data, f, indent=2)
