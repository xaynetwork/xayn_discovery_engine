use ::tracing::{event, Level};
use diesel::{prelude::*, sql_query};

mod schema {
    diesel::table! {
        posts (id) {
            id -> Integer,
            title -> Text,
            body -> Text,
            published -> Bool,
        }
    }
}

use schema::posts;

#[derive(Queryable, Debug)]
pub(crate) struct Post {
    id: i32,
    title: String,
    body: String,
    published: bool,
}

#[derive(Insertable, Debug)]
#[table_name = "posts"]
struct NewPost<'a> {
    title: &'a str,
    body: &'a str,
}

pub(crate) fn run_database_demo(db_path: &String) {
    event!(Level::INFO, "\n\n [SQLite] trying to open DB\n\n");
    event!(Level::INFO, "\n\n [SQLite] path: {}\n\n", db_path);

    let conn = establish_connection(db_path);
    event!(Level::INFO, "\n\n [SQLite] DB opened!!\n\n");

    setup_table(&conn);
    event!(Level::INFO, "\n\n [SQLite] Table setup successful!!\n\n");

    create_post(&conn, "Hello world", "What a strange world it is");
    event!(Level::INFO, "\n\n [SQLite] Insert post successful!!\n\n");

    let results = show_posts(&conn, true);
    event!(
        Level::INFO,
        "\n\n [SQLite] Found {} published posts!!\n\n",
        results.len()
    );

    let results = show_posts(&conn, false);
    for post in results {
        publish_posts(&conn, post.id);
    }

    let results = show_posts(&conn, true);
    event!(
        Level::INFO,
        "\n\n [SQLite] Found {} published posts!!\n\n",
        results.len()
    );
}

pub(crate) fn establish_connection(db_path: &String) -> SqliteConnection {
    SqliteConnection::establish(&db_path)
        .unwrap_or_else(|_| panic!("Error connecting to {}", db_path))
}

pub(crate) fn setup_table(conn: &SqliteConnection) {
    sql_query(
        "CREATE TABLE IF NOT EXISTS posts (
            id INTEGER NOT NULL PRIMARY KEY,
            title VARCHAR NOT NULL,
            body TEXT NOT NULL,
            published BOOLEAN NOT NULL DEFAULT 0
        )",
    )
    .execute(conn)
    .expect("Table creation failed");
}

pub(crate) fn create_post(conn: &SqliteConnection, title: &str, body: &str) -> usize {
    let new_post = NewPost { title, body };

    event!(
        Level::INFO,
        "\n\n [SQLite] create_post: {:#?}\n\n",
        new_post
    );

    diesel::insert_into(posts::table)
        .values(&new_post)
        .execute(conn)
        .expect("Error saving new post")
}

pub(crate) fn publish_posts(conn: &SqliteConnection, id: i32) {
    use self::posts::dsl::{posts, published};

    let _ = diesel::update(posts.find(id))
        .set(published.eq(true))
        .execute(conn)
        .unwrap_or_else(|_| panic!("Unable to find post {}", id));

    let post: Post = posts
        .find(id)
        .first(conn)
        .unwrap_or_else(|_| panic!("Unable to find post {}", id));

    event!(Level::INFO, "\n\n [SQLite] Published post: {:#?}\n\n", post);
}

pub(crate) fn show_posts(conn: &SqliteConnection, published_status: bool) -> Vec<Post> {
    use self::posts::dsl::{posts, published};

    let results = posts
        .filter(published.eq(published_status))
        .limit(5)
        .load::<Post>(conn)
        .expect("Error loading posts");

    results
}
