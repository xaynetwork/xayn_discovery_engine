{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.xayn.private"},
        "path": {"$match":"*"},
        "name": {"$match":"*"},
        "$or": [
          {
            "$and": [
              {
                "created" : {"$last" : "1d"}
              }
            ]
          }
        ]
      }
    }
  }
]
}

