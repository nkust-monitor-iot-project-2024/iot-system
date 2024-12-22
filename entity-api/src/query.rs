use async_graphql::SimpleObject;
use async_graphql::types::connection::*;

use crate::entity::Entity;
use crate::prelude::*;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a list of monitors.
    async fn monitors(&self, context: &Context<'_>) -> async_graphql::Result<Vec<Monitor>> {
        let pool = context.data::<DatabasePool>()?.get_pool();

        let mut monitors: Vec<_> = sqlx::query_as!(Monitor, "SELECT id FROM monitors")
            .fetch_all(&pool)
            .await?;

        // Add a monitor with no ID to represent the entities without the monitor.
        monitors.push(Monitor { id: None });

        Ok(monitors)
    }

    /// Get a monitor by ID.
    /// If the ID is `None`, it means the monitor is not specified.
    async fn monitor(
        &self,
        context: &Context<'_>,
        id: Option<String>,
    ) -> async_graphql::Result<Monitor> {
        let pool = context.data::<DatabasePool>()?.get_pool();

        let monitor = sqlx::query_as!(Monitor, "SELECT id FROM monitors WHERE id = $1", id)
            .fetch_optional(&pool)
            .await?;

        Ok(monitor.unwrap_or(Monitor { id }))
    }

    /// Get an entity by ID.
    async fn entity(&self, context: &Context<'_>, id: i32) -> async_graphql::Result<Entity> {
        let pool = context.data::<DatabasePool>()?.get_pool();

        let entity = sqlx::query_as!(
            Entity,
            "SELECT id, image_id, label, confidence, monitor_id, created_at FROM entities WHERE id = $1",
            id
        )
        .fetch_one(&pool)
        .await?;

        Ok(entity)
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Monitor {
    /// The ID of the monitor.
    ///
    /// [`Option::None`] means no monitor specified.
    pub id: Option<String>,
}

#[async_graphql::ComplexObject]
impl Monitor {
    /// Get a list of entities.
    ///
    /// The default pagination stragety is to get the last 10 entities.
    /// You can use `first` and `last` arguments to customize the pagination.
    /// If you want to get the next page, you should provide the `after` (for `first`)
    /// or `before` (for `last`) cursor.
    async fn entities(
        &self,
        context: &Context<'_>,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> async_graphql::Result<Connection<i32, Entity, EmptyFields, EmptyFields>> {
        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let pool = context.data::<DatabasePool>()?.get_pool();

                if let Some(first) = first {
                    let mut entities = sqlx::query_as!(
                        Entity,
                        r#"
                    SELECT
                        id,
                        image_id,
                        label,
                        confidence,
                        monitor_id,
                        created_at
                    FROM entities
                    WHERE (
                        ($1::text IS NOT NULL AND monitor_id = $1)
                        OR
                        ($1::text IS NULL AND monitor_id IS NULL)
                    )
                    AND id > $2
                    ORDER BY id ASC
                    LIMIT $3
                "#,
                        self.id,
                        after.unwrap_or(0),
                        first as i64 + 1
                    )
                    .fetch_all(&pool)
                    .await?;

                    let has_next_page = entities.len() > first;
                    let mut connection = Connection::new(after.unwrap_or(0) > 0, has_next_page);

                    entities.truncate(first as usize);

                    connection.edges.extend(
                        entities
                            .into_iter()
                            .map(|entity| Edge::new(entity.id, entity)),
                    );

                    Ok::<_, async_graphql::Error>(connection)
                } else {
                    // default strategy: descending order, last first
                    let last = last.unwrap_or(10);

                    let mut entities = sqlx::query_as!(
                        Entity,
                        "
                    SELECT
                        id,
                        image_id,
                        label,
                        confidence,
                        monitor_id,
                        created_at
                    FROM entities
                    WHERE (
                        ($1::text IS NOT NULL AND monitor_id = $1)
                        OR
                        ($1::text IS NULL AND monitor_id IS NULL)
                    )
                    AND id < $2
                    ORDER BY id DESC
                    LIMIT $3
                ",
                        self.id,
                        before.unwrap_or(i32::MAX),
                        last as i64 + 1
                    )
                    .fetch_all(&pool)
                    .await?;

                    let has_next_page = entities.len() > last;
                    let mut connection =
                        Connection::new(before.unwrap_or(i32::MAX) < i32::MAX, has_next_page);

                    entities.truncate(last);

                    connection.edges.extend(
                        entities
                            .into_iter()
                            .map(|entity| Edge::new(entity.id, entity)),
                    );

                    Ok::<_, async_graphql::Error>(connection)
                }
            },
        )
        .await
    }
}
