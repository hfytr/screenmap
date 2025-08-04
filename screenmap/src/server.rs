use crate::interface::{ColType, CysQuery, ScreenmapRow};
use leptos::{prelude::ServerFnError, server};
use std::{fmt::Debug, ops::RangeInclusive};

type ServerFnResult<T> = Result<T, ServerFnError>;

cfg_if::cfg_if! {
if #[cfg(feature = "ssr")] {
    use anyhow::{Context, Error};
    use axum::extract::FromRef;
    use leptos::{
        config::LeptosOptions,
        prelude::{use_context},
    };
    use sqlx::{PgPool, Row, postgres::PgRow};
    use std::sync::Arc;

    #[derive(Clone, FromRef)]
    pub struct AppState {
        pub leptos_options: LeptosOptions,
        pub pool: Arc<PgPool>,
    }

    impl AppState {
        pub fn from_cx() -> ServerFnResult<Self> {
            use_context().ok_or(ServerFnError::new("Failed to provide context"))
        }
    }
}}

#[server(name = CysLocation, prefix = "/api")]
pub async fn cys_location(cys_query: CysQuery) -> ServerFnResult<i64> {
    let state = AppState::from_cx()?;
    let get_result = async move || {
        let cys_col_name = sqlx::query(
            r#"
        SELECT column_name 
        FROM information_schema.columns 
        WHERE table_name = $1
        ORDER BY ordinal_position
        "#,
        )
        .bind(&cys_query.screen_name)
        .try_map(|row: sqlx::postgres::PgRow| row.try_get::<String, _>("column_name"))
        .fetch_all(state.pool.as_ref())
        .await?;

        sqlx::query("SELECT id FROM $1 WHERE $2 = $3")
            .bind(&cys_query.screen_name)
            .bind(&cys_col_name[0])
            .bind(&cys_query.cys_name)
            .fetch_optional(state.pool.as_ref())
            .await?
            .map(|row| {
                row.try_get::<i64, &str>("id")
                    .context("Failed getting 'id' from table.")
            })
            .ok_or(Error::msg(
                "ERROR: Given cysteine name not found in database.",
            ))?
    };
    get_result().await.map_err(ServerFnError::new)
}

#[server(name = Search, prefix = "/api")]
pub async fn search_tbls(query: String) -> ServerFnResult<Vec<String>> {
    use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
    let state = AppState::from_cx()?;
    let tbls = get_tbls(&state).await?;
    let matcher = SkimMatcherV2::default();
    let mut scored_tbls: Vec<_> = tbls
        .into_iter()
        .filter_map(|s| matcher.fuzzy_match(&s, &query).map(|x| (s, x)))
        .collect();
    scored_tbls.sort_by(|(_, x0), (_, x1)| x0.cmp(x1));
    Ok(scored_tbls.into_iter().map(|(s, _)| s).collect())
}

#[server(name = GetNumRows, prefix = "/api")]
pub async fn get_num_rows(tbl_name: String) -> ServerFnResult<usize> {
    let state = AppState::from_cx()?;
    if !get_tbls(&state).await?.contains(&tbl_name) {
        return Err(ServerFnError::new("Nice try."));
    }
    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", tbl_name))
        .fetch_one(&*state.pool)
        .await?;
    Ok(count as usize)
}

#[server(name = SearchTable, prefix = "/api")]
pub async fn search_table(
    tbl_name: String,
    query: String,
) -> Result<Vec<RangeInclusive<usize>>, ServerFnError> {
    let state = AppState::from_cx()?;
    if !get_tbls(&state).await?.contains(&tbl_name) {
        return Err(ServerFnError::new("Nice try."));
    } else if query.is_empty() {
        return Ok(vec![1..=get_num_rows(tbl_name).await?]);
    }
    let columns = get_screen_keys(tbl_name.clone()).await?;
    let where_clause = columns
        .iter()
        .map(|col| format!("{}::TEXT ILIKE $1", col.0))
        .collect::<Vec<_>>()
        .join(" OR ");

    let sql = format!(
        "SELECT id FROM {} WHERE {}",
        tbl_name,
        if where_clause.is_empty() {
            "TRUE".into()
        } else {
            where_clause
        }
    );

    let pattern = format!("%{}%", query);
    let row_ids: Vec<i32> = sqlx::query_scalar(&sql)
        .bind(pattern)
        .fetch_all(&*state.pool)
        .await
        .map_err(ServerFnError::new)?;

    if row_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut sorted_ids: Vec<_> = row_ids.into_iter().map(|i| i as usize).collect();
    sorted_ids.sort_unstable();
    sorted_ids.dedup();

    let mut ranges = vec![];
    let mut start = sorted_ids[0];
    let mut end = sorted_ids[0];

    for &id in sorted_ids.iter().skip(1) {
        if id == end + 1 {
            end = id;
        } else {
            ranges.push(start..=end);
            start = id;
            end = id;
        }
    }
    ranges.push(start..=end);

    Ok(ranges)
}

#[server(name = GetRows, prefix = "/api")]
pub async fn get_rows(
    rows: Vec<usize>,
    tbl_name: String,
) -> ServerFnResult<Vec<(usize, ScreenmapRow)>> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    let state = AppState::from_cx()?;
    let screen_keys = get_screen_keys_inner(&tbl_name, &state).await;
    let params = (1..=rows.len())
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(",");

    let query_str = format!(
        "SELECT * FROM {} WHERE id IN ({}) ORDER BY id",
        tbl_name, params
    );
    let mut query = sqlx::query(&query_str);
    for id in rows.iter() {
        query = query.bind(*id as i32);
    }

    let fetched_rows = query.fetch_all(&*state.pool).await?;
    let result = rows
        .into_iter()
        .zip(fetched_rows.into_iter())
        .map(|(row_id, row)| {
            screen_keys
                .clone()?
                .into_iter()
                .map(|(col, col_type)| match col_type {
                    ColType::TEXT => get_col::<String>(&row, col),
                    ColType::REAL => get_col::<f64>(&row, col),
                    ColType::DOUBLE => get_col::<f32>(&row, col),
                    ColType::SMALLINT => get_col::<i16>(&row, col),
                    ColType::INT => get_col::<i32>(&row, col),
                    ColType::BIGINT => get_col::<i64>(&row, col),
                })
                .try_collect()
                .map(|row| (row_id, row))
        })
        .map(|x| x)
        .try_collect()?;
    Ok(result)
}

#[cfg(feature = "ssr")]
fn get_col<'b, 'a: 'b, T>(row: &'a PgRow, col: String) -> ServerFnResult<(String, String)>
where
    T: ToString + sqlx::Type<sqlx::Postgres> + sqlx::Decode<'b, sqlx::Postgres> + Debug + Default,
{
    let res = row
        .try_get(col.as_str())
        .map_err(ServerFnError::new)
        .map(|t: Option<T>| (col.clone(), t.unwrap_or_default().to_string()));
    res
}

#[cfg(feature = "ssr")]
async fn get_tbls(state: &AppState) -> ServerFnResult<Vec<String>> {
    sqlx::query_scalar(
        r#"
        SELECT table_name
        FROM information_schema.tables
        WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
          AND table_type = 'BASE TABLE'
        ORDER BY table_name
        "#,
    )
    .fetch_all(state.pool.as_ref())
    .await
    .map_err(ServerFnError::new)
}

#[server(name = ScreenKeys, prefix = "/api")]
pub async fn get_screen_keys(screen_name: String) -> ServerFnResult<Vec<(String, ColType)>> {
    let state = AppState::from_cx()?;
    get_screen_keys_inner(&screen_name, &state).await
}

#[cfg(feature = "ssr")]
async fn get_screen_keys_inner(
    screen_name: &str,
    state: &AppState,
) -> ServerFnResult<Vec<(String, ColType)>> {
    sqlx::query(
        r#"
        SELECT column_name, data_type
        FROM information_schema.columns
        WHERE table_name = $1
        AND table_schema = current_schema()
        ORDER BY ordinal_position
    "#,
    )
    .bind(screen_name)
    .try_map(|row: PgRow| {
        Ok((
            row.try_get::<String, _>("column_name")?,
            ColType::from_str(&row.try_get::<String, _>("data_type")?).unwrap_or(ColType::TEXT),
        ))
    })
    .fetch_all(state.pool.as_ref())
    .await
    .map_err(ServerFnError::new)
}
