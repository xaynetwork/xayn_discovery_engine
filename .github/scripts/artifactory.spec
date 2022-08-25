{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.xayn.private"},
        "path": {"$nmatch":"main"},
        "name": {"$match":"*"},
        "$or": [
          {
            "$and": [
              {
                "created" : {"$before" : "5d"}
              }
            ]
          }
        ]
      }
    }
  }
]
}

