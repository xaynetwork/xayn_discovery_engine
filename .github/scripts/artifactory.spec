{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.yellow.private"},
        "path": {"$nmatch":"*main*"},
        "name": {"$match":"*"}
      }
    }
  }
]
}

