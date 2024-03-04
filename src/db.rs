use axum::async_trait;
use serde::Deserialize;
use sqlx::FromRow;

use crate::{
    errors::AppError,
    model::{Owner, ProjectID, ProjectDataPatch, ProjectDataPost, User, Users},
    pagination::{Direction, SortBy},
    version::Version
};

#[derive(Debug, Deserialize, FromRow, PartialEq)]
pub struct ProjectSummaryRow {
    pub rank: f64,
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub revision: i64,
    pub created_at: i64,
    pub modified_at: i64,
    pub game_title: String,
    pub game_title_sort: String,
    pub game_publisher: String,
    pub game_year: String,
    pub image: Option<String>
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ProjectRow {
    pub project_id: i64,
    pub name: String,
    pub description: String,
    pub revision: i64,
    pub created_at: i64,
    pub modified_at: i64,
    pub modified_by: i64,
    pub game_title: String,
    pub game_title_sort: String,
    pub game_publisher: String,
    pub game_year: String,
    pub image: Option<String>,
    pub readme: String
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct PackageRow {
    pub package_id: i64,
    pub name: String,
    pub created_at: i64
//    description: String
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ReleaseRow {
    pub release_id: i64,
    pub version: String,
    pub version_major: i64,
    pub version_minor: i64,
    pub version_patch: i64,
    pub version_pre: String,
    pub version_build: String,
    pub filename: String,
    pub url: String,
    pub size: i64,
    pub checksum: String,
    pub published_at: i64,
    pub published_by: String
//    requires: String
}

#[async_trait]
pub trait DatabaseClient {
    async fn get_project_id(
        &self,
        _project: &str
    ) -> Result<ProjectID, AppError>;

    async fn get_projects_count(
        &self,
    ) -> Result<i64, AppError>;

    async fn get_projects_query_count(
        &self,
        _query: &str
    ) -> Result<i64, AppError>;

    async fn get_user_id(
        &self,
        _user: &str
    ) -> Result<i64, AppError>;

    async fn get_owners(
        &self,
        _proj_id: i64
    ) -> Result<Users, AppError>;

    async fn user_is_owner(
        &self,
        _user: &User,
        _proj_id: i64
    ) -> Result<bool, AppError>;

    async fn add_owner(
        &self,
        _user_id: i64,
        _proj_id: i64
    ) -> Result<(), AppError>;

    async fn add_owners(
        &self,
        _owners: &Users,
        _proj_id: i64
    ) -> Result<(), AppError>;

    async fn remove_owner(
        &self,
        _user_id: i64,
        _proj_id: i64
    ) -> Result<(), AppError>;

    async fn remove_owners(
        &self,
        _owners: &Users,
        _proj_id: i64
    ) -> Result<(), AppError>;

    async fn has_owner(
        &self,
        _proj_id: i64,
    ) -> Result<bool, AppError>;

    async fn get_projects_end_window(
        &self,
        _sort_by: SortBy,
        _dir: Direction,
        _limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>;

    async fn get_projects_query_end_window(
        &self,
        _query: &str,
        _sort_by: SortBy,
        _dir: Direction,
        _limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>;

    async fn get_projects_mid_window(
        &self,
        _sort_by: SortBy,
        _dir: Direction,
        _field: &str,
        _id: u32,
        _limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>;

    async fn get_projects_query_mid_window(
        &self,
        _query: &str,
        _sort_by: SortBy,
        _dir: Direction,
        _field: &str,
        _id: u32,
        _limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>;

    async fn create_project(
        &self,
        _user: &User,
        _proj: &str,
        _proj_data: &ProjectDataPost,
        _now: i64
    ) -> Result<(), AppError>;

    async fn update_project(
        &self,
        _owner: &Owner,
        _proj_id: i64,
        _proj_data: &ProjectDataPatch,
        _now: i64
    ) -> Result<(), AppError>;

    async fn get_project_row(
        &self,
        _proj_id: i64
    ) -> Result<ProjectRow, AppError>;

    async fn get_project_row_revision(
        &self,
        _proj_id: i64,
        _revision: i64
    ) -> Result<ProjectRow, AppError>;

    async fn get_packages(
        &self,
        _proj_id: i64
    ) -> Result<Vec<PackageRow>, AppError>;

    async fn get_packages_at(
        &self,
        _proj_id: i64,
        _date: i64,
    ) -> Result<Vec<PackageRow>, AppError>;

    async fn get_releases(
        &self,
        _pkg_id: i64
    ) -> Result<Vec<ReleaseRow>, AppError>;

    async fn get_releases_at(
        &self,
        _pkg_id: i64,
        _date: i64
    ) -> Result<Vec<ReleaseRow>, AppError>;

    async fn get_authors(
        &self,
        _pkg_ver_id: i64
    ) -> Result<Users, AppError>;

    async fn get_package_url(
        &self,
        _pkg_id: i64
    ) -> Result<String, AppError>;

    async fn get_release_url(
        &self,
        _pkg_id: i64,
        _version: &Version
    ) -> Result<String, AppError>;

    async fn get_players(
        &self,
        _proj_id: i64
    ) -> Result<Users, AppError>;

    async fn add_player(
        &self,
        _player: &User,
        _proj_id: i64,
    ) -> Result<(), AppError>;

    async fn remove_player(
        &self,
        _player: &User,
        _proj_id: i64
    ) -> Result<(), AppError>;

    async fn get_image_url(
        &self,
        _proj_id: i64,
        _img_name: &str
    ) -> Result<String, AppError>;
}
