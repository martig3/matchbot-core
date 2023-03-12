use crate::maps::Map;
use crate::matches::SeriesType::Bo1;
use crate::matches::SeriesType::Bo3;
use crate::matches::SeriesType::Bo5;
use crate::matches::VoteType::Veto;
use crate::team::Team;
use crate::tournament::Tournament;
use anyhow::bail;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use sqlx::PgPool;
use sqlx::{FromRow, PgExecutor, Type};
use std::str::FromStr;
use std::{fmt, i32};
use strum::EnumIter;

#[allow(unused)]
#[derive(Debug, FromRow)]
pub struct Match {
    pub id: i32,
    pub match_series: i32,
    pub map: i32,
    pub picked_by: i32,
    pub start_ct_team: i32,
    pub start_t_team: i32,
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone)]
pub struct NewMatch {
    pub map_id: i32,
    pub picked_by_role: i64,
    pub start_ct_team_role: Option<i64>,
    pub start_t_team_role: Option<i64>,
}

#[allow(unused)]
#[derive(Debug, FromRow, Serialize)]
pub struct Server {
    pub id: i32,
    pub match_series: i32,
    pub server_id: String,
    pub hostname: String,
    pub game_port: i32,
    pub gotv_port: i32,
}

impl Server {
    pub async fn add(
        executor: impl PgExecutor<'_>,
        match_series: i32,
        server_id: &String,
        hostname: &String,
        game_port: i32,
        gotv_port: i32,
    ) -> Result<bool> {
        let result = sqlx::query!(
            "insert into servers (match_series, server_id, hostname, game_port, gotv_port)\
            values ($1, $2, $3, $4, $5)",
            match_series,
            server_id,
            hostname,
            game_port,
            gotv_port,
        )
        .execute(executor)
        .await?;
        let rows_affected = result.rows_affected() == 1;
        Ok(rows_affected)
    }
    pub async fn get_live(executor: impl PgExecutor<'_>) -> Result<Vec<Server>> {
        Ok(sqlx::query_as!(
            Server,
            "select s.* from servers s \
                join match_series ms on ms.id = s.match_series \
            where ms.completed_at is null",
        )
        .fetch_all(executor)
        .await?)
    }
}

#[derive(Deserialize, Debug)]
pub struct DathostSeriesResponse {
    pub matches: Vec<DathostMatch>,
}

#[derive(Deserialize, Debug)]
pub struct DathostMatch {
    pub id: String,
    pub map: String,
}

#[allow(unused)]
#[derive(Debug, FromRow, Serialize)]
pub struct MatchSeries {
    pub id: i32,
    pub team_one: i32,
    pub team_two: i32,
    pub series_type: SeriesType,
    pub dathost_match: Option<String>,
    pub created_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub thread: Option<i64>,
}

#[derive(Debug, FromRow)]
pub struct VoteInfo {
    pub id: i32,
    pub match_series: i32,
    pub map: i32,
    #[sqlx(rename = "type")]
    pub vote_type: VoteType,
    pub team: i32,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Type, EnumIter,
)]
#[sqlx(rename_all = "lowercase", type_name = "series_type")]
pub enum SeriesType {
    Bo1,
    Bo3,
    Bo5,
}

impl fmt::Display for SeriesType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bo1 => write!(f, "bo1"),
            Bo3 => write!(f, "bo3"),
            Bo5 => write!(f, "bo5"),
        }
    }
}

impl FromStr for SeriesType {
    type Err = ();
    fn from_str(input: &str) -> Result<SeriesType, Self::Err> {
        match input {
            "bo1" => Ok(Bo1),
            "bo3" => Ok(Bo3),
            "bo5" => Ok(Bo5),
            _ => Err(()),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, Type,
)]
#[sqlx(rename_all = "lowercase", type_name = "vote_type")]
pub enum VoteType {
    Veto,
    Pick,
}

#[allow(unused)]
impl fmt::Display for VoteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VoteType::Veto => write!(f, "Veto"),
            VoteType::Pick => write!(f, "Pick"),
        }
    }
}

#[derive(FromRow, Serialize)]
#[allow(unused)]
pub struct MatchScore {
    pub id: i32,
    pub match_id: i32,
    pub team_one_score: i32,
    pub team_two_score: i32,
}

impl MatchScore {
    pub async fn add(executor: impl PgExecutor<'_>, match_series: i32) -> Result<bool> {
        let result = sqlx::query!(
            "
            insert into match_scores (match_id)
                    VALUES
                        ($1)
                    ",
            match_series,
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected() == 1)
    }
    pub async fn get_by_series(
        executor: impl PgExecutor<'_>,
        match_series: i32,
    ) -> Result<Vec<MatchScore>> {
        Ok(sqlx::query_as!(
            MatchScore,
            "select ms.*
                 from match_scores ms
                    join match m on ms.match_id = m.id
                    join match_series mts on m.match_series = mts.id
                where mts.id = $1 
                order by ms.id",
            match_series,
        )
        .fetch_all(executor)
        .await?)
    }
}

impl VoteInfo {
    pub async fn add(
        executor: impl PgExecutor<'_>,
        match_series: i32,
        map: i32,
        vote_type: VoteType,
        team: i32,
    ) -> Result<VoteInfo> {
        Ok(sqlx::query_as(
            "
            insert into vote_info (match_series, map, type, team)
                    VALUES
                        ($1, $2, $3, $4)
                    RETURNING *",
        )
        .bind(match_series)
        .bind(map)
        .bind(vote_type)
        .bind(team)
        .fetch_one(executor)
        .await?)
    }
    pub async fn get_by_match_series(
        executor: impl PgExecutor<'_>,
        match_series: i32,
    ) -> Result<Vec<VoteInfo>> {
        Ok(
            sqlx::query_as("select * from vote_info where match_series = $1 order by id")
                .bind(match_series)
                .fetch_all(executor)
                .await?,
        )
    }
}

impl MatchSeries {
    pub async fn create(
        executor: impl PgExecutor<'_>,
        team_one: i32,
        team_two: i32,
        series_type: SeriesType,
        tournament: Tournament,
    ) -> Result<MatchSeries> {
        Ok(sqlx::query_as(
            "INSERT INTO match_series
                        (team_one, team_two, series_type, created_at, tournament)
                    VALUES
                        ($1, $2, $3, now(), $4)
                    RETURNING *",
        )
        .bind(team_one)
        .bind(team_two)
        .bind(series_type)
        .bind(tournament.id)
        .fetch_one(executor)
        .await?)
    }
    pub async fn get(executor: impl PgExecutor<'_>, id: i32) -> Result<Option<MatchSeries>> {
        Ok(
            sqlx::query_as(format!("select * from match_series where id = $1",).as_str())
                .bind(id)
                .fetch_optional(executor)
                .await?,
        )
    }
    pub async fn get_all(
        executor: &PgPool,
        limit: u64,
        completed: bool,
        tournament_id: Option<i32>,
    ) -> Result<Vec<MatchSeries>> {
        let completed_clause = if completed { "is not null" } else { "is null" };
        let tournament = match tournament_id {
            Some(id) => Some(Tournament::get(executor, id).await?),
            None => Tournament::get_current(executor).await?,
        };
        let Some(tournament) = tournament else {
            bail!("No current tournament");
        };
        Ok(sqlx::query_as(
            format!(
                "select ms.* from match_series ms
                    where completed_at {} and ms.tournament = $2
                 order by id desc limit $1",
                completed_clause,
            )
            .as_str(),
        )
        .bind(limit as i64)
        .bind(tournament.id as i64)
        .fetch_all(executor)
        .await?)
    }

    pub async fn get_scheduled(executor: impl PgExecutor<'_>) -> Result<Vec<MatchSeries>> {
        Ok(sqlx::query_as(
            "select * from match_series
                    where completed_at is null 
                      and dathost_match is null
                 order by id",
        )
        .fetch_all(executor)
        .await?)
    }

    pub async fn update_dathost_match(
        &self,
        executor: impl PgExecutor<'_>,
        dathost_match_id: String,
    ) -> Result<bool> {
        let result = sqlx::query!(
            "UPDATE match_series SET dathost_match = $1 WHERE id = $2",
            dathost_match_id,
            self.id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected() == 1)
    }
    pub async fn update_thread(&self, executor: impl PgExecutor<'_>, thread: i64) -> Result<bool> {
        let result = sqlx::query!(
            "UPDATE match_series SET thread = $1 WHERE id = $2",
            thread,
            self.id
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected() == 1)
    }
    pub async fn next_user_match(executor: impl PgExecutor<'_>, user: i64) -> Result<MatchSeries> {
        Ok(sqlx::query_as(
            "
                select ms.*
                from match_series ms
                    join teams t on t.id = ms.team_one or t.id = ms.team_two
                    join team_members tm on t.id = tm.team
                    join steam_ids si on si.discord = tm.member
                where si.discord = $1
                    and ms.completed_at is null
                order by ms.id",
        )
        .bind(user)
        .fetch_one(executor)
        .await?)
    }

    pub async fn get_all_by_user(
        executor: impl PgExecutor<'_>,
        limit: u64,
        user: u64,
        completed: bool,
    ) -> Result<Vec<MatchSeries>> {
        let completed_clause = if completed { "is not null" } else { "is null" };
        Ok(sqlx::query_as(
            format!(
                "select ms.*
                    from match_series ms
                    join teams t on t.id = ms.team_one or ms.team_two = t.id
                    join team_members tm on t.id = tm.team
                where tm.member = $2
                    and ms.completed_at {}
                order by ms.id desc
                limit $1",
                completed_clause
            )
            .as_str(),
        )
        .bind(limit as i64)
        .bind(user as i64)
        .fetch_all(executor)
        .await?)
    }
    pub async fn delete(executor: impl PgExecutor<'_>, id: i32) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM match_series where id = $1", id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn get_in_progress(executor: impl PgExecutor<'_>) -> Result<Vec<MatchSeries>> {
        Ok(sqlx::query_as(
            "select distinct ms.*
                 from match_series ms
                   join match m on m.match_series = ms.id
                   join match_scores mi on m.id = mi.match_id
                   join servers s on s.match_series = ms.id
                 where ms.completed_at is null
                   and m.completed_at is null
                 ",
        )
        .fetch_all(executor)
        .await?)
    }
    pub async fn veto_info(
        &self,
        pool: &PgPool,
        vote_info: Option<Vec<VoteInfo>>,
    ) -> Result<String> {
        let vote_info = if vote_info.is_none() {
            VoteInfo::get_by_match_series(pool, self.id).await?
        } else {
            vote_info.unwrap()
        };
        if vote_info.is_empty() {
            return Ok(String::from("This match has no veto info yet"));
        }
        let team_one = Team::get(pool, self.team_one).await.unwrap();
        let team_two = Team::get(pool, self.team_two).await.unwrap();
        let maps = Map::get_all(pool, false).await.unwrap();
        let mut info_string = String::from("```diff\n");
        let rows: String = vote_info
            .into_iter()
            .filter(|v| v.map > 0)
            .map(|v| {
                let mut row_str = String::new();
                let team_name = if v.team == team_one.id {
                    &team_one.name
                } else {
                    &team_two.name
                };
                let map_name = &maps.iter().find(|m| m.id == v.map).unwrap().name;
                if v.vote_type == Veto {
                    row_str.push_str(format!("- {} banned {}\n", team_name, map_name,).as_str());
                } else {
                    row_str.push_str(format!("+ {} picked {}\n", team_name, map_name,).as_str());
                }
                row_str
            })
            .collect();
        info_string.push_str(rows.as_str());
        info_string.push_str("```");
        Ok(info_string)
    }
}

impl Match {
    pub async fn create(
        executor: impl PgExecutor<'_>,
        match_series: i32,
        map: i32,
        picked_by: i32,
        start_ct_team: i32,
        start_t_team: i32,
    ) -> Result<Match> {
        Ok(sqlx::query_as(
            "INSERT INTO match 
                        (match_series, map, picked_by, start_ct_team, start_t_team)
                    VALUES
                        ($1, $2, $3, $4, $5)
                    RETURNING *",
        )
        .bind(match_series)
        .bind(map)
        .bind(picked_by)
        .bind(start_ct_team)
        .bind(start_t_team)
        .fetch_one(executor)
        .await?)
    }
    pub async fn get_by_series(
        executor: impl PgExecutor<'_>,
        match_series: i32,
    ) -> Result<Vec<Match>> {
        Ok(sqlx::query_as!(
            Match,
            "select m.* from match_series ms
                join match m on m.match_series = ms.id
                where match_series = $1
                order by m.id
                ",
            match_series
        )
        .fetch_all(executor)
        .await?)
    }
}
