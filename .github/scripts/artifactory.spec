{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.xayn.private"},
        "path": {"$match":"/([\s\S]*?)(main)/g"},
        "name": {"$match":"*"},
        "$or": [
          {
            "$and": [
              {
                "created" : {"$last" : "10d"}
              }
            ]
          }
        ]
      }
    }
  }
]
}

