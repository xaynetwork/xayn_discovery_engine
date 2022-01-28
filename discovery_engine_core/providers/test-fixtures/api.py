#!/usr/bin/env python3

import requests
import json
import os

headers = {
    "x-api-key": os.environ['NEWSCATCHER_API_KEY'],
}

url = "https://api.newscatcherapi.com/v2/search"

queries = {
    "climate-change": {"q":'"Climate change"',"sort_by":"relevancy", "page_size": "2", "lang": "en", "countries": "AU"},
    "msft-vs-aapl": {"q":'("Bill Gates" || "Tim Cook")',"sort_by":"relevancy", "page_size": "2", "lang": "de", "countries": "DE"},
}

for (key, params) in queries.items():
    response = requests.request("GET", url, headers=headers, params=params)
    data = json.loads(response.text)

    for article in data['articles']:
        article['author'] = 'Anonymous'
        article['link'] = "https://xayn.com"
        article['clean_url'] = 'xayn.com'
        article['rights'] = 'xayn.com'
        article['authors'] = ['Anonymous']
        article['media'] = "https://uploads-ssl.webflow.com/5ea197660b956f76d26f0026/6179684043a88260009773cd_hero-phone.png"
        article['twitter_account'] = "@XaynHQ"

    
    with open(f"{key}.json", 'w') as f:
        json.dump(data, f, indent=2)
