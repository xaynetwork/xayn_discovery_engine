{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.xayn.private"},
        "path": {"$match":"*"},
        "name": {"$match":"*#main"},
        "$or": [
          {
            "$and": [
              {
              }
            ]
          }
        ]
      }
    }
  }
]
}

