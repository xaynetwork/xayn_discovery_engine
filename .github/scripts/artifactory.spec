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
                #"created": { "$before":"number_days_limit" }
              }
            ]
          }
        ]
      }
    }
  }
]
}

