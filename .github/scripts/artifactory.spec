{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.yellow.private"},
        "path": {"$nmatch":"*main*"},
        "name": {"$match":"*main*"},
        "$or": [
            {
              "$and": [
                {
                  "created" : {"$before":"number_days_limit"}
                }
            ]
          }
        ]
      }
    }
  }
]
}

