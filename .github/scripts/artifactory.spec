{
"files": [
  {
    "aql": {
      "items.find": {
        "repo": {"$eq":"dart.yellow.private"},
        "path": {"$nmatch":"*main*"},
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

