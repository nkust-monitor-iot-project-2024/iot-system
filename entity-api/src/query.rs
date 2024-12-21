use async_graphql::types::connection::*;

use crate::entity::Entity;
use crate::prelude::*;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn entities(
        &self,
        context: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> async_graphql::Result<Connection<i32, Entity, EmptyFields, EmptyFields>> {
        query(after, before, first, last, |after, before, first, last| async move {
            let pool = get_pgpool(context)?;

            if let Some(first) = first {
                tracing::info!("after: {:?}, first: {:?}", after, first);
                let mut entities = sqlx::query_as!(Entity, "SELECT id, image_id, label, confidence, created_at FROM entities WHERE id > $1 ORDER BY id ASC LIMIT $2", after.unwrap_or(0), first as i64 + 1)
                    .fetch_all(&pool)
                    .await?;

                let has_next_page = entities.len() > first;
                let mut connection = Connection::new(after.unwrap_or(0) > 0, has_next_page);

                entities.truncate(first as usize);

                connection.edges.extend(
                    entities.into_iter().map(|entity|
                        Edge::new(entity.id, entity)
                ));

                Ok::<_, async_graphql::Error>(connection)
            } else {
                // default strategy: descending order, last first
                let last = last.unwrap_or(10);

                let mut entities = sqlx::query_as!(Entity, "SELECT id, image_id, label, confidence, created_at FROM entities WHERE id < $1 ORDER BY id DESC LIMIT $2", before.unwrap_or(i32::MAX), last as i64 + 1)
                    .fetch_all(&pool)
                    .await?;

                let has_next_page = entities.len() > last;
                let mut connection = Connection::new(before.unwrap_or(i32::MAX) < i32::MAX, has_next_page);

                entities.truncate(last);

                connection.edges.extend(
                    entities.into_iter().map(|entity|
                        Edge::new(entity.id, entity)
                ));

                Ok::<_, async_graphql::Error>(connection)
            }
        }).await
    }

    async fn entity(&self, context: &Context<'_>, id: i32) -> async_graphql::Result<Entity> {
        let pool = get_pgpool(context)?;

        let entity = sqlx::query_as!(
            Entity,
            "SELECT id, image_id, label, confidence, created_at FROM entities WHERE id = $1",
            id
        )
        .fetch_one(&pool)
        .await?;

        Ok(entity)
    }
}
