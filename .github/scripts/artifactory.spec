{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.yellow.private"},
        "path": {"$match":"*"},
        "name": {"$match":"*"},
        "$or": [
          {
            "$and": [
              {
                "created": { "$before":"number_days_limit" }
              }
            ]
          }
        ]
      }
    }
  }
]
}

