#![feature(async_fn_in_trait)]

use axum::{
    Router, Server,
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get
};
//use base64::{Engine, engine::general_purpose};
use jsonwebtoken::DecodingKey;
use serde_json::json;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::net::SocketAddr;

mod config;
mod db;
mod errors;
mod extractors;
mod jwt;
mod model;

use crate::{
    config::Config,
    errors::AppError,
    model::{Owner, Users},
    db::{Database, add_owners, get_owners, remove_owners}
};

#[derive(Clone, FromRef)]
struct AppState {
    key: jwt::Key, 
    database: Database 
}

/*
struct HttpError {
    status: u16,
    message: String
}
*/

/*
impl From<base64::DecodeError> for AppError {
    fn from(e: base64::DecodeError) -> Self {
        AppError::BadPagination
    }
}
*/

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
/*
            AppError::BadPagination => {
                (StatusCode::BAD_REQUEST, "Bad pagination".into())
            },
*/
            AppError::DatabaseError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e)
            },
            AppError::InternalError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".into())
            },
            AppError::NotImplemented => {
                (StatusCode::NOT_IMPLEMENTED, "Not implemented".into())
            },
            AppError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized".into())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

async fn root() -> &'static str {
    "hello world"
}

/*
async fn projects<D: Database>(Query(pagination): Query<Pagination>, Extension(db_pool): Extension<Pool<D>>) -> Result<Json<String>, AppError> {
    let result = sqlx::query_as!(
        Project,
        "
SELECT id, game_title
FROM modules
WHERE id > ?
ORDER BY game_title_sort_key
COLLATE NOCASE LIMIT ?
        ",
        pagination.cursor as i64,
        pagination.count as i64
    ) 
    .fetch_all(&db_pool)
    .await?;

    println!("{:?}", result); 

    Ok(Json("".into())) 
}
*/

/*
async fn projects<DB: Database>(Query(pagination): Query<Pagination>, Extension(db_pool): Extension<Pool<DB>>) -> Result<Json<Vec<Project>>, AppError> {

    let result = sqlx::query_as::<_, Project>(
        "
SELECT id, game_title
FROM modules
WHERE id > ?
ORDER BY game_title_sort_key
COLLATE NOCASE LIMIT ?
        "
    )
    .bind(pagination.cursor.unwrap_or(0) as u32)
    .bind(pagination.count.unwrap_or(100) as u32)
    .fetch_all(&db_pool)
    .await?;

    Ok(Json(result))
}
*/

/*
fn app<D: Database>(config: &Config, db_pool: Pool<D>) -> Router {
    let base = &config.api_base_path;
    Router::new()
        .route(&format!("{base}/"), get(root))
        .route(&format!("{base}/projects"), get(projects::<D>))
        .layer(Extension(db_pool))
}
*/

/*
#[derive(Deserialize)]
struct Pagination {
    seek: Option<String>,
    count: Option<u32>,
}

#[derive(Debug, FromRow, Serialize)]
struct Project {
    id: i64,
    game_title: Option<String>,
    game_title_sort_key: Option<String>
}

#[derive(Debug, Serialize)]
struct ProjectsMeta {
    prev_page: Option<String>,
    next_page: Option<String>,
    total: usize
}

#[derive(Debug, Serialize)]
struct Projects {
    projects: Vec<Project>,
    meta: ProjectsMeta
}

fn decode_seek(s: &str) -> Result<String, AppError> {
    String::from_utf8(
        general_purpose::URL_SAFE_NO_PAD.decode(&s)
            .or(Err(AppError::BadPagination))?
    ).or(Err(AppError::BadPagination))
}

fn encode_seek(s: &str) -> String {
    return general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes());
} 

async fn projects(Query(pagination): Query<Pagination>, Extension(db_pool): Extension<SqlitePool>) -> Result<Json<Projects>, AppError> {

    let seek = match pagination.seek {
        Some(s) => decode_seek(&s)?,
        None => "".into()
    };

    println!("{}", seek);

    let rows = sqlx::query_as::<_, Project>(
        "
SELECT id, game_title, game_title_sort_key
FROM modules
WHERE game_title_sort_key > ?
ORDER BY game_title_sort_key
COLLATE NOCASE LIMIT ?
        "
    )
    .bind(seek)
    .bind(pagination.count.unwrap_or(100))
    .fetch_all(&db_pool)
    .await?;

    let next = match rows.len() {
        0 => None,
        l => rows[l-1].game_title_sort_key.as_ref().map(|k| encode_seek(&k))
    };

    let result = Projects {
        meta: ProjectsMeta {
            prev_page: None,
            next_page: next, 
            total: 0
        },
        projects: rows
    };

    Ok(Json(result))
}
*/

async fn owners_get(
    Path(proj_id): Path<u32>,
    State(db): State<Database>
) -> Result<Json<Users>, AppError>
{
    Ok(Json(get_owners(proj_id, &db).await?))
}

async fn owners_add(
    requester: Owner,
    Path(proj_id): Path<u32>,
    State(db): State<Database>,
    Json(owners): Json<Vec<String>>
) -> Result<(), AppError> {
    add_owners(&owners, proj_id, &db).await
}

async fn owners_remove(
    requester: Owner,
    Path(proj_id): Path<u32>,
    State(db): State<Database>,
    Json(owners): Json<Vec<String>>
) -> Result<(), AppError>
{
    remove_owners(&owners, proj_id, &db).await
}

fn app(config: &Config, db_pool: SqlitePool) -> Router {
    let api = &config.api_base_path;

    let state = AppState {
        key: jwt::Key(DecodingKey::from_secret(&config.jwt_key)),
        database: Database(db_pool)
    };

    Router::new()
        .route(&format!("{api}/"), get(root))
//        .route(&format!("{base}/projects"), get(projects))
//        .route(&format!("{base}/projects/:proj_id"), get(project_read).put(project_update))
//        .route(&format!("{base}/projects/:proj_id/:revision"), get(project_revision))
        .route(
            &format!("{api}/projects/:proj_id/owners"),
            get(owners_get).put(owners_add).delete(owners_remove)
        )
//        .route(&format!("{base}/projects/:proj_id/players"), get(project_players_get).put(project_players_add).delete(project_players_remove))
//        .route(&format!("{base}/projects/:proj_id/packages/:pkg_name"), get(project_package_read))
//        .route(&format!("{base}/projects/:proj_id/packages/:pkg_name/:version"), get(project_package_version_read).put(project_package_version_write))
//        .route(&format!("{base}/projects/:proj_id/readme"), get(project_readme))
//        .route(&format!("{base}/projects/:proj_id//readme/:revision"), get(project_readme_revision))
//        .route(&format!("{base}/projects/:proj_id//images/:img_name"), get(project_image_get).put(project_image_put))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let config = Config {
        db_path: "projects.db".into(),
// TODO: read key from file? env?
        jwt_key: b"@wlD+3L)EHdv28u)OFWx@83_*TxhVf9IdUncaAz6ICbM~)j+dH=sR2^LXp(tW31z".to_vec(),
        api_base_path: "/api/v1".into(),
        listen_ip: [0, 0, 0, 0],
        listen_port: 3000
    };

// TODO: handle error?
    let db_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite://{}", &config.db_path))
        .await
        .unwrap();

    let app = app(&config, db_pool);

    let addr = SocketAddr::from((config.listen_ip, config.listen_port));
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    use super::*;


    




}