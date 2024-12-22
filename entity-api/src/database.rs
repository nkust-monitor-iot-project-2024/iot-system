#[derive(Clone)]
pub struct DatabasePool(sqlx::PgPool);

impl DatabasePool {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = sqlx::PgPool::connect(url).await?;

        Ok(Self(pool))
    }

    pub fn get_pool(&self) -> sqlx::PgPool {
        self.0.clone()
    }
}
