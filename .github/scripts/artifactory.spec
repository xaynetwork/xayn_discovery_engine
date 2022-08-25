{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.yellow.private"},
        "path": {"$nmatch":"*"},
        "name": {"$nmatch":"*main*"},
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

