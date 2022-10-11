use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct Response<T> {
    pub hits: Hits<T>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Hits<T> {
    pub hits: Vec<Hit<T>>,
    pub total: Total,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Hit<T> {
    #[serde(rename(deserialize = "_id"))]
    pub id: String,
    #[serde(rename(deserialize = "_source"))]
    pub source: T,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Article {
    pub title: String,
    pub excerpt: String,
    pub clean_url: String,
    pub link: String,
    pub topic: String,
    pub country: String,
    pub language: String,
    pub published_date: String,
    pub embedding: Vec<f32>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Total {
    pub value: usize,
}
