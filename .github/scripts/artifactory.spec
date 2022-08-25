{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.xayn.private"},
        "path": {"$match":"*#main"},
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

