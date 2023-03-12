use anyhow::{bail, Result};
use serde::Serialize;
use sqlx::PgPool;
use sqlx::{FromRow, PgExecutor};
use std::i32;

use crate::tournament::Tournament;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Team {
    pub id: i32,
    pub role: i64,
    pub name: String,
    pub captain: i64,
    pub tournament: i32,
    r#is_active: Option<bool>,
}

#[allow(unused)]
impl Team {
    pub async fn create(
        executor: impl PgExecutor<'_>,
        role: i64,
        name: &str,
        captain: i64,
        tournament_id: i32,
    ) -> Result<Team> {
        Ok(sqlx::query_as!(
            Team,
            "INSERT INTO teams
                        (role, name, captain, tournament, is_active)
                    VALUES
                        ($1, $2, $3, $4, true)
                    RETURNING *",
            role,
            name,
            captain,
            tournament_id,
        )
        .fetch_one(executor)
        .await?)
    }

    pub async fn get(executor: impl PgExecutor<'_>, id: i32) -> Result<Team> {
        Ok(
            sqlx::query_as!(Team, "SELECT * FROM TEAMS where id = $1", id)
                .fetch_one(executor)
                .await?,
        )
    }

    pub async fn get_all(executor: impl PgExecutor<'_>) -> Result<Vec<Team>> {
        Ok(
            sqlx::query_as!(Team, "SELECT * FROM TEAMS where is_active is true",)
                .fetch_all(executor)
                .await?,
        )
    }

    pub async fn delete(executor: impl PgExecutor<'_>, team: i32) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM teams WHERE id = $1", team)
            .execute(executor)
            .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn add_member(executor: impl PgExecutor<'_>, team: i32, member: i64) -> Result<bool> {
        let result = sqlx::query!(
            "INSERT INTO team_members (team, member) VALUES ($1, $2)",
            team,
            member
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn remove_member(
        executor: impl PgExecutor<'_>,
        team: i32,
        member: i64,
    ) -> Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM team_members WHERE team = $1 AND member = $2",
            team,
            member
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn get_by_role(executor: impl PgExecutor<'_>, role: i64) -> Result<Option<Team>> {
        Ok(
            sqlx::query_as!(Team, "SELECT * FROM teams WHERE role = $1", role)
                .fetch_optional(executor)
                .await?,
        )
    }

    pub async fn get_by_member(executor: impl PgExecutor<'_>, member: i64) -> Result<Option<Team>> {
        Ok(sqlx::query_as!(
            Team,
            "SELECT teams.*
                     FROM team_members
                     JOIN teams
                        ON team_members.team = teams.id
                     WHERE team_members.member = $1
                        and is_active is true",
            member
        )
        .fetch_optional(executor)
        .await?)
    }

    pub async fn members(&self, executor: impl PgExecutor<'_>) -> Result<Vec<i64>> {
        Ok(
            sqlx::query_scalar!("SELECT member FROM team_members WHERE team = $1", self.id)
                .fetch_all(executor)
                .await?,
        )
    }

    pub async fn update_captain(
        executor: impl PgExecutor<'_>,
        team: i32,
        member: i64,
    ) -> Result<bool> {
        let result = sqlx::query!("UPDATE teams SET captain = $1 WHERE id = $2", member, team)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() == 1)
    }
    pub async fn format_team_str(&self, mut members: Vec<i64>) -> String {
        members.retain(|member| *member != self.captain as i64);

        let captain = format!("<@{captain}>", captain = self.captain);
        let members = members
            .into_iter()
            .map(|member| format!("<@{member}>"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Team <@&{role}>\n\tCaptain: {captain}\n\tMembers: {members}",
            role = self.role
        )
    }
    pub async fn set_inactive(&self, executor: impl PgExecutor<'_>) -> Result<bool> {
        let result = sqlx::query!("UPDATE teams SET is_active = false WHERE id = $1", self.id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() == 1)
    }
}

pub async fn create_team(
    pool: &PgPool,
    role: u64,
    name: impl AsRef<str>,
    captain: u64,
) -> Result<()> {
    let tournament = Tournament::get_current(pool).await?;
    let Some(tournament) = tournament else {
        bail!("There is no active tournament to sign up for.");
    };
    let mut transaction = pool.begin().await?;
    let team = Team::create(
        &mut transaction,
        role as i64,
        name.as_ref(),
        captain as i64,
        tournament.id,
    )
    .await?;
    Team::add_member(&mut transaction, team.id, captain as i64).await?;
    transaction.commit().await?;
    Ok(())
}
