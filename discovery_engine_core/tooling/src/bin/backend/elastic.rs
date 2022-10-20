use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct CountResponse {
    pub count: usize,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SearchResponse<T> {
    pub hits: Hits<T>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Hits<T> {
    pub hits: Vec<Hit<T>>,
    // pub total: Total,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Hit<T> {
    #[serde(rename(deserialize = "_id"))]
    pub id: String,
    #[serde(rename(deserialize = "_source"))]
    pub source: T,
    pub sort: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Article {
    #[serde(rename(deserialize = "Title"))]
    pub title: String,
    #[serde(rename(deserialize = "Abstract"))]
    pub snippet: String,
    #[serde(rename(deserialize = "URL"))]
    pub url: String,
    #[serde(rename(deserialize = "Category"))]
    pub category: String,
    pub date_published: String,
}

#[allow(dead_code)]
#[derive(Clone, Deserialize, Debug)]
pub struct Total {
    pub value: usize,
}
