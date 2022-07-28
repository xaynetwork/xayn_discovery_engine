#Artifactory.spec is an AQL (Artifactory Query Language) file
#This file is use to manage artifacts and builds stored within Jfrog's Artifactory 
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
                "created": { "$before":"dddd" }
              }
            ]
          }
        ]
      }
    }
  }
]
}
