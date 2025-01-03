pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod entity;
pub(crate) mod mutation;
pub(crate) mod prelude;
pub(crate) mod query;
pub(crate) mod storage;

use anyhow::Context as _;
use async_graphql::{EmptySubscription, Schema, http::GraphiQLSource};
use async_graphql_poem::GraphQL;
use mutation::MutationRoot;
use poem::{IntoResponse, Route, Server, get, handler, listener::TcpListener, web::Html};
use prelude::DatabasePool;
use query::QueryRoot;
use storage::Storage;

use std::{net::SocketAddr, str::FromStr};

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").finish())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::parse_config()?;

    let storage = Storage::from_config(config.s3).context("Failed to connect to the S3 storage")?;

    let pool = DatabasePool::connect(&config.database_url)
        .await
        .context("Failed to connect to the database")?;

    let addr = SocketAddr::from_str(&std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:8080".into()))
        .context("Invalid BIND_ADDR")?;

    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .data(storage)
        .finish();

    let app = Route::new().at("/", get(graphiql).post(GraphQL::new(schema)));

    tracing::info!(
        "To access the GraphQL playground, visit http://127.0.0.1:{port}",
        port = addr.port()
    );

    Server::new(TcpListener::bind(addr)).run(app).await.unwrap();

    Ok(())
}
