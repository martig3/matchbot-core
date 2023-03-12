use anyhow::Result;
use sqlx::{FromRow, PgExecutor};
use time::OffsetDateTime;

#[derive(Debug, FromRow, Clone)]
pub struct Tournament {
    pub id: i32,
    pub name: String,
    pub started_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
}
impl Tournament {
    pub async fn get(executor: impl PgExecutor<'_>, id: i32) -> Result<Tournament> {
        Ok(
            sqlx::query_as!(Tournament, "select * from tournament where id = $1", id)
                .fetch_one(executor)
                .await?,
        )
    }
    pub async fn get_all(executor: impl PgExecutor<'_>) -> Result<Vec<Tournament>> {
        Ok(
            sqlx::query_as!(Tournament, "select * from tournament order by id desc",)
                .fetch_all(executor)
                .await?,
        )
    }
    pub async fn get_current(executor: impl PgExecutor<'_>) -> Result<Option<Tournament>> {
        Ok(Tournament::get_all(executor)
            .await?
            .into_iter()
            .filter(|t| t.completed_at.is_none())
            .collect::<Vec<Tournament>>()
            .first()
            .cloned())
    }
    pub async fn create(self, executor: impl PgExecutor<'_>) -> Result<Tournament> {
        let result = sqlx::query_as!(Tournament,
            "insert into tournament (name, started_at, completed_at) values ($1, $2, null) returning *", 
            self.name, self.started_at)
            .fetch_one(executor)
            .await?;
        Ok(result)
    }
    pub async fn update(self, executor: impl PgExecutor<'_>) -> Result<Tournament> {
        let result = sqlx::query_as!(Tournament,
            "update tournament SET name = $1, completed_at = $2, started_at = $3 where tournament.id = $4 returning *", 
            self.name, self.started_at, self.completed_at, self.id)
            .fetch_one(executor)
            .await?;
        Ok(result)
    }
}
