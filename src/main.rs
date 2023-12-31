#![feature(async_fn_in_trait)]

use axum::{
    Router, serve,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post, put}
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use std::{
    net::SocketAddr,
    sync::Arc
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

mod app;
mod config;
mod core;
mod db;
mod errors;
mod extractors;
mod handlers;
mod jwt;
mod model;
mod pagination;
mod prod_core;
mod sqlite;
mod version;

use crate::{
    app::AppState,
    config::Config,
    core::CoreArc,
    prod_core::ProdCore,
    errors::AppError,
    jwt::DecodingKey,
    sqlite::SqlxDatabaseClient
};

impl From<&AppError> for StatusCode {
    fn from(err: &AppError) -> Self {
        match err {
            AppError::BadMimeType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            AppError::CannotRemoveLastOwner => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::JsonError => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::LimitOutOfRange => StatusCode::BAD_REQUEST,
            AppError::MalformedQuery => StatusCode::BAD_REQUEST,
            AppError::MalformedVersion => StatusCode::BAD_REQUEST,
            AppError::NotAPackage => StatusCode::NOT_FOUND,
            AppError::NotAProject => StatusCode::NOT_FOUND,
            AppError::NotARevision => StatusCode::NOT_FOUND,
            AppError::NotAUser => StatusCode::NOT_FOUND,
            AppError::NotAVersion => StatusCode::NOT_FOUND,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct HttpError {
    error: String
}

impl From<AppError> for HttpError {
    fn from(err: AppError) -> Self {
        HttpError { error: format!("{}", err) }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = StatusCode::from(&self);
        let body = Json(HttpError::from(self));
        (code, body).into_response()
    }
}

fn routes(api: &str) -> Router<AppState> {
    Router::new()
        .route(
            &format!("{api}/"),
            get(handlers::root_get)
        )
        .route(
            &format!("{api}/projects"),
            get(handlers::projects_get)
        )
        .route(
            &format!("{api}/projects/:proj"),
            get(handlers::project_get)
            .put(handlers::project_put)
        )
        .route(
            &format!("{api}/projects/:proj/:revision"),
            get(handlers::project_revision_get)
        )
        .route(
            &format!("{api}/projects/:proj/owners"),
            get(handlers::owners_get)
            .put(handlers::owners_add)
            .delete(handlers::owners_remove)
        )
        .route(
            &format!("{api}/projects/:proj/players"),
            get(handlers::players_get)
            .put(handlers::players_add)
            .delete(handlers::players_remove)
        )
        .route(
            &format!("{api}/projects/:proj/packages"),
            put(handlers::packages_put)
        )
        .route(
            &format!("{api}/projects/:proj/packages/:pkg_name"),
            get(handlers::release_get)
        )
        .route(
            &format!("{api}/projects/:proj/packages/:pkg_name/:version"),
            get(handlers::release_version_get)
            .put(handlers::release_put)
        )
        .route(
            &format!("{api}/projects/:proj/images/:img_name"),
            get(handlers::image_get)
            .put(handlers::image_put)
        )
        .route(
            &format!("{api}/projects/:proj/flag"),
            post(handlers::flag_post)
        )
        .route(
            &format!("{api}/readme/:readme_id"),
            get(handlers::readme_get)
        )
        .fallback(handlers::not_found)
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::very_permissive())
        )
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

    let core = ProdCore {
        db: SqlxDatabaseClient(db_pool),
        now: Utc::now
    };

    let state = AppState {
        key: DecodingKey::from_secret(&config.jwt_key),
        core: Arc::new(core) as CoreArc
    };

    let api = &config.api_base_path;

    let app: Router = routes(api)
        .with_state(state);

    let addr = SocketAddr::from((config.listen_ip, config.listen_port));
    let listener = TcpListener::bind(addr).await.unwrap();
    serve(listener, app)
        .await
        .unwrap();
}

#[cfg(test)]
mod test {
    use super::*;

    use axum::{
        body::{self, Body, Bytes},
        http::{
            Method, Request,
            header::{AUTHORIZATION, CONTENT_TYPE, LOCATION}
        }
    };
    use mime::{APPLICATION_JSON, TEXT_PLAIN};
    use once_cell::sync::Lazy;
    use tower::ServiceExt; // for oneshot

    use crate::{
        core::Core,
        jwt::{self, EncodingKey},
        model::{GameData, PackageData, PackageID, Project, ProjectData, ProjectDataPut, ProjectID, Projects, ProjectSummary, Readme, ReleaseData, User, Users},
        pagination::{Limit, Pagination, Seek, SeekLink},
        version::Version
    };

    const API_V1: &str = "/api/v1";
    const KEY: &[u8] = b"@wlD+3L)EHdv28u)OFWx@83_*TxhVf9IdUncaAz6ICbM~)j+dH=sR2^LXp(tW31z";

    async fn body_bytes(r: Response) -> Bytes {
        body::to_bytes(r.into_body(), usize::MAX).await.unwrap()
    }

    async fn body_as<D: for<'a> Deserialize<'a>>(r: Response) -> D {
        serde_json::from_slice::<D>(&body_bytes(r).await).unwrap()
    }

    async fn body_empty(r: Response) -> bool {
        body_bytes(r).await.is_empty()
    }

// TODO: fill in the fields
// TODO: can these be declared some other way?
    static PROJECT_SUMMARY_A: Lazy<ProjectSummary> = Lazy::new(|| {
        ProjectSummary {
            name: "project_a".into(),
            description: "".into(),
            revision: 1,
            created_at: "".into(),
            modified_at: "".into(),
            tags: vec![],
            game: GameData {
                title: "".into(),
                title_sort_key: "".into(),
                publisher: "".into(),
                year: "".into()
            }
        }
    });

    static PROJECT_SUMMARY_B: Lazy<ProjectSummary> = Lazy::new(|| {
        ProjectSummary {
            name: "project_b".into(),
            description: "".into(),
            revision: 1,
            created_at: "".into(),
            modified_at: "".into(),
            tags: vec![],
            game: GameData {
                title: "".into(),
                title_sort_key: "".into(),
                publisher: "".into(),
                year: "".into()
            }
        }
    });

    #[derive(Clone)]
    struct TestCore { }

    #[axum::async_trait]
    impl Core for TestCore {
        async fn get_project_id(
            &self,
            proj: &Project
        ) -> Result<ProjectID, AppError>
        {
            match proj.0.as_str() {
                "a_project" => Ok(ProjectID(1)),
                _ => Err(AppError::NotAProject)
            }
        }

        async fn get_package_id(
            &self,
            _proj_id: i64,
            pkg: &str
        ) -> Result<PackageID, AppError>
        {
            match pkg {
                "a_package" => Ok(PackageID(1)),
                _ => Err(AppError::NotAPackage)
            }
        }

        async fn user_is_owner(
            &self,
            user: &User,
            _proj_id: i64
        ) -> Result<bool, AppError>
        {
            Ok(user == &User("bob".into()) || user == &User("alice".into()))
        }

        async fn add_owners(
            &self,
            _owners: &Users,
            _proj_id: i64
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn remove_owners(
            &self,
            _owners: &Users,
            _proj_id: i64
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn get_owners(
            &self,
            _proj_id: i64
        ) -> Result<Users, AppError>
        {
            Ok(
                Users {
                    users: vec![
                        User("alice".into()),
                        User("bob".into())
                    ]
                }
            )
        }

        async fn get_projects(
            &self,
            _from: Seek,
            _limit: Limit
        ) -> Result<Projects, AppError>
        {
            Ok(
                Projects {
                    projects: vec![
                        PROJECT_SUMMARY_A.clone(),
                        PROJECT_SUMMARY_B.clone()
                    ],
                    meta: Pagination {
                        prev_page: Some(
                            SeekLink::new(Seek::Before("project_a".into()))
                        ),
                        next_page: Some(
                            SeekLink::new(Seek::After("project_b".into()))
                        ),
                        total: 1234
                    }
                }
            )
        }

        async fn get_project(
            &self,
            _proj_id: i64,
        ) -> Result<ProjectData, AppError>
        {
            Ok(
                ProjectData {
                    name: "eia".into(),
                    description: "A module for Empires in Arms".into(),
                    revision: 1,
                    created_at: "2023-10-26T00:00:00,000000000+01:00".into(),
                    modified_at: "2023-10-30T18:53:53,056386142+00:00".into(),
                    tags: vec![],
                    game: GameData {
                        title: "Empires in Arms".into(),
                        title_sort_key: "Empires in Arms".into(),
                        publisher: "Avalon Hill".into(),
                        year: "1983".into()
                    },
                    readme_id: 3,
                    owners: vec!["alice".into(), "bob".into()],
                    packages: vec![
                        PackageData {
                            name: "".into(),
                            description: "".into(),
                            releases: vec![
                                ReleaseData {
                                    version: "".into(),
                                    filename: "".into(),
                                    url: "".into(),
                                    size: 0,
                                    checksum: "".into(),
                                    published_at: "".into(),
                                    published_by: "".into(),
                                    requires: "".into(),
                                    authors: vec![]
                                }
                            ]
                        }
                    ]
                }
            )
        }

        async fn create_project(
            &self,
            _user: &User,
            _proj: &str,
            _proj_data: &ProjectDataPut
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn update_project(
            &self,
            _proj_id: i64,
            _proj_data: &ProjectDataPut
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn get_project_revision(
            &self,
            proj_id: i64,
            revision: u32
        ) -> Result<ProjectData, AppError>
        {
            match revision {
                1 => self.get_project(proj_id).await,
                _ => Err(AppError::NotARevision)
            }
        }

        async fn get_release(
            &self,
            _proj_id: i64,
            _pkg_id: i64
        ) -> Result<String, AppError>
        {
            Ok("https://example.com/package".into())
        }

        async fn get_release_version(
            &self,
            _proj_id: i64,
            _pkg_id: i64,
            version: &Version
        ) -> Result<String, AppError>
        {
            match version {
                Version { major: 1, minor: 2, patch: 3, .. } => {
                    Ok("https://example.com/package-1.2.3".into())
                },
                _ => Err(AppError::NotAVersion)
            }
        }

        async fn get_players(
            &self,
            _proj_id: i64
        ) -> Result<Users, AppError>
        {
            Ok(
                Users {
                    users: vec![
                        User("player 1".into()),
                        User("player 2".into())
                    ]
                }
            )
        }

        async fn add_player(
            &self,
            _player: &User,
            _proj_id: i64
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn remove_player(
            &self,
            _player: &User,
            _proj_id: i64
        ) -> Result<(), AppError>
        {
            Ok(())
        }

        async fn get_readme(
            &self,
            readme_id: i64
        ) -> Result<Readme, AppError>
        {

            match readme_id {
                42 => Ok(Readme { text: "Stuff!".into() }),
                _ => Err(AppError::NotFound)
            }
        }

        async fn get_image(
            &self,
            proj_id: i64,
            img_name: &str
        ) -> Result<String, AppError>
        {
            if proj_id == 1 && img_name == "img.png" {
                Ok("https://example.com/img.png".into())
            }
            else {
                Err(AppError::NotFound)
            }
        }
    }

    fn test_state() -> AppState {
        AppState {
            key: DecodingKey::from_secret(KEY),
            core: Arc::new(TestCore {}) as CoreArc
        }
    }

    fn token(user: &str) -> String {
        let ekey = EncodingKey::from_secret(KEY);
        let token = jwt::issue(&ekey, user, 899999999999).unwrap();
        format!("Bearer {token}")
    }

    async fn try_request(request: Request<Body>) -> Response {
        routes(API_V1)
            .with_state(test_state())
            .oneshot(request)
            .await
            .unwrap()
    }

    fn headers<'a>(
        response: &'a Response,
        header_name: &str
    ) -> Vec<&'a [u8]>
    {
        let mut values = response
            .headers()
            .get_all(header_name)
            .iter()
            .map(|v| v.as_ref())
            .collect::<Vec<_>>();

        values.sort_unstable();
        values
    }

    #[tokio::test]
    async fn cors_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);

        assert_eq!(
            headers(&response, "access-control-allow-credentials"),
            ["true".as_bytes()]
        );

        assert_eq!(
            headers(&response, "vary"),
            [
                "access-control-request-headers".as_bytes(),
                "access-control-request-method".as_bytes(),
                "origin".as_bytes()
            ]
        );
    }

    #[tokio::test]
    async fn root_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response).await[..], b"hello world");
    }

    #[tokio::test]
    async fn get_projects_no_params_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_limit_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit=5"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_limit_zero() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit=0"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::LimitOutOfRange)
        );
    }

    #[tokio::test]
    async fn get_projects_limit_too_large() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit=100000"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::LimitOutOfRange)
        );
    }

    #[tokio::test]
    async fn get_projects_limit_empty() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit="))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::LimitOutOfRange)
        );
    }

    #[tokio::test]
    async fn get_projects_limit_not_a_number() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit=eleventeen"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::LimitOutOfRange)
        );
    }

    #[tokio::test]
    async fn get_projects_seek_start_ok() {
        let seek = String::from(Seek::Start);

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek={seek}"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_seek_end_ok() {
        let seek = String::from(Seek::End);

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek={seek}"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_seek_before_ok() {
        let seek = String::from(Seek::Before("xyz".into()));

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek={seek}"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_seek_after_ok() {
        let seek = String::from(Seek::After("xyz".into()));

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek={seek}"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_seek_empty() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek="))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::MalformedQuery)
        );
    }

    #[tokio::test]
    async fn get_projects_seek_bad() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek=%@$"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::MalformedQuery)
        );
    }

// TODO: seek string too long?

    #[tokio::test]
    async fn get_projects_seek_and_limit_ok() {
        let seek = String::from(Seek::Start);

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?seek={seek}&limit=5"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_projects_limit_and_seek_ok() {
        let seek = String::from(Seek::Start);

        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects?limit=5&seek={seek}"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Projects>(response).await,
            Projects {
                projects: vec![
                    PROJECT_SUMMARY_A.clone(),
                    PROJECT_SUMMARY_B.clone()
                ],
                meta: Pagination {
                    prev_page: Some(
                        SeekLink::new(Seek::Before("project_a".into()))
                    ),
                    next_page: Some(
                        SeekLink::new(Seek::After("project_b".into()))
                    ),
                    total: 1234
                }
            }
        );
    }

    #[tokio::test]
    async fn get_project_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<ProjectData>(response).await,
            ProjectData {
                name: "eia".into(),
                description: "A module for Empires in Arms".into(),
                revision: 1,
                created_at: "2023-10-26T00:00:00,000000000+01:00".into(),
                modified_at: "2023-10-30T18:53:53,056386142+00:00".into(),
                tags: vec![],
                game: GameData {
                    title: "Empires in Arms".into(),
                    title_sort_key: "Empires in Arms".into(),
                    publisher: "Avalon Hill".into(),
                    year: "1983".into()
                },
                readme_id: 3,
                owners: vec!["alice".into(), "bob".into()],
// TODO: fill in more
                packages: vec![
                    PackageData {
                        name: "".into(),
                        description: "".into(),
                        releases: vec![
                            ReleaseData {
                                version: "".into(),
                                filename: "".into(),
                                url: "".into(),
                                size: 0,
                                checksum: "".into(),
                                published_at: "".into(),
                                published_by: "".into(),
                                requires: "".into(),
                                authors: vec![]
                            }
                        ]
                    }
                ]
            }
        );
    }

    #[tokio::test]
    async fn get_project_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn put_project_create() {
        let proj_data = ProjectDataPut {
            description: "A module for Empires in Arms".into(),
            tags: vec![],
            game: GameData {
                title: "Empires in Arms".into(),
                title_sort_key: "Empires in Arms".into(),
                publisher: "Avalon Hill".into(),
                year: "1983".into()
            }
        };

        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_vec(&proj_data).unwrap()))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn put_project_update() {
        let proj_data = ProjectDataPut {
            description: "A module for Empires in Arms".into(),
            tags: vec![],
            game: GameData {
                title: "Empires in Arms".into(),
                title_sort_key: "Empires in Arms".into(),
                publisher: "Avalon Hill".into(),
                year: "1983".into()
            }
        };

        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_vec(&proj_data).unwrap()))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn put_project_unauth() {
        let proj_data = ProjectDataPut {
            description: "A module for Empires in Arms".into(),
            tags: vec![],
            game: GameData {
                title: "Empires in Arms".into(),
                title_sort_key: "Empires in Arms".into(),
                publisher: "Avalon Hill".into(),
                year: "1983".into()
            }
        };

        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(serde_json::to_vec(&proj_data).unwrap()))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn put_project_wrong_json() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .header(AUTHORIZATION, token("bob"))
                .body(Body::from(r#"{ "garbage": "whatever" }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::JsonError)
        );
    }

    #[tokio::test]
    async fn put_project_wrong_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .header(CONTENT_TYPE, TEXT_PLAIN.as_ref())
                .header(AUTHORIZATION, token("bob"))
                .body(Body::from("stuff"))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn put_project_no_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project"))
                .header(CONTENT_TYPE, TEXT_PLAIN.as_ref())
                .header(AUTHORIZATION, token("bob"))
                .body(Body::from("stuff"))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn get_project_revision_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/1"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<ProjectData>(response).await,
            ProjectData {
                name: "eia".into(),
                description: "A module for Empires in Arms".into(),
                revision: 1,
                created_at: "2023-10-26T00:00:00,000000000+01:00".into(),
                modified_at: "2023-10-30T18:53:53,056386142+00:00".into(),
                tags: vec![],
                game: GameData {
                    title: "Empires in Arms".into(),
                    title_sort_key: "Empires in Arms".into(),
                    publisher: "Avalon Hill".into(),
                    year: "1983".into()
                },
                readme_id: 3,
                owners: vec!["alice".into(), "bob".into()],
                packages: vec![
                    PackageData {
                        name: "".into(),
                        description: "".into(),
                        releases: vec![
                            ReleaseData {
                                version: "".into(),
                                filename: "".into(),
                                url: "".into(),
                                size: 0,
                                checksum: "".into(),
                                published_at: "".into(),
                                published_by: "".into(),
                                requires: "".into(),
                                authors: vec![]
                            }
                        ]
                    }
                ]
            }
        );
    }

    #[tokio::test]
    async fn get_project_revision_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/1"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn get_project_revision_not_a_revision() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/2"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotARevision)
        );
    }

    #[tokio::test]
    async fn get_package_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/packages/a_package"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap(),
            "https://example.com/package"
        );
    }

    #[tokio::test]
    async fn get_package_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/packages/a_package"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn get_package_not_a_package() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/packages/not_a_package"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAPackage)
        );
    }

    #[tokio::test]
    async fn get_release_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/packages/a_package/1.2.3"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap(),
            "https://example.com/package-1.2.3"
        );
    }

    #[tokio::test]
    async fn get_release_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/packages/a_package/1.2.3"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn get_release_not_a_package() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/packages/not_a_package/1.2.3"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAPackage)
        );
    }

    #[tokio::test]
    async fn get_release_not_a_version() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/packages/a_package/bogus"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAVersion)
        );
    }

    #[tokio::test]
    async fn get_owners_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Users>(response).await,
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

    #[tokio::test]
    async fn get_owners_bad_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/owners"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn put_owners_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn put_owners_bad_project() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn put_owners_unauth() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("rando"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn put_owners_wrong_json() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "garbage": "whatever" }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::JsonError)
        );
    }

    #[tokio::test]
    async fn put_owners_wrong_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, TEXT_PLAIN.as_ref())
                .body(Body::from("stuff"))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn put_owners_no_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn delete_owners_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn delete_owners_bad_project() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/not_a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn delete_owners_unauth() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("rando"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::Unauthorized)
        );
    }

    #[tokio::test]
    async fn delete_owners_wrong_json() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, APPLICATION_JSON.as_ref())
                .body(Body::from(r#"{ "garbage": "whatever" }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::JsonError)
        );
    }

    #[tokio::test]
    async fn delete_owners_wrong_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .header(CONTENT_TYPE, TEXT_PLAIN.as_ref())
                .body(Body::from("stuff"))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn delete_owners_no_mime_type() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .body(Body::from(r#"{ "users": ["alice", "bob"] }"#))
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::BadMimeType)
        );
    }

    #[tokio::test]
    async fn get_players_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/players"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Users>(response).await,
            Users {
                users: vec![
                    User("player 1".into()),
                    User("player 2".into())
                ]
            }
        );
    }

    #[tokio::test]
    async fn get_players_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/players"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn put_players_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/a_project/players"))
                .header(AUTHORIZATION, token("bob"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn put_players_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::PUT)
                .uri(&format!("{API_V1}/projects/not_a_project/owners"))
                .header(AUTHORIZATION, token("bob"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn delete_players_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/a_project/players"))
                .header(AUTHORIZATION, token("player 1"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert!(body_empty(response).await);
    }

    #[tokio::test]
    async fn delete_players_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::DELETE)
                .uri(&format!("{API_V1}/projects/not_a_project/players"))
                .header(AUTHORIZATION, token("player 1"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }

    #[tokio::test]
    async fn get_readme_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/readme/42"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_as::<Readme>(response).await,
            Readme { text: "Stuff!".into() }
        );
    }

    #[tokio::test]
    async fn get_readme_not_an_id() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/readme/1"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotFound)
        );
    }

    #[tokio::test]
    async fn get_image_ok() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/a_project/images/img.png"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(LOCATION).unwrap(),
            "https://example.com/img.png"
        );
    }

    #[tokio::test]
    async fn get_image_not_a_project() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/projects/not_a_project/images/img.png"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotAProject)
        );
    }


    #[tokio::test]
    async fn bad_path() {
        let response = try_request(
            Request::builder()
                .method(Method::GET)
                .uri(&format!("{API_V1}/bogus/whatever"))
                .body(Body::empty())
                .unwrap()
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            body_as::<HttpError>(response).await,
            HttpError::from(AppError::NotFound)
        );
    }
}
