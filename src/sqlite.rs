use axum::async_trait;
use sqlx::{
    Acquire, Database, Executor, QueryBuilder,
    sqlite::Sqlite
};
use serde::Deserialize;
use std::cmp::Ordering;

use crate::{
    db::{DatabaseClient, PackageRow, ProjectRow, ProjectSummaryRow, ReleaseRow},
    errors::AppError,
    model::{Owner, ProjectID, ProjectDataPatch, ProjectDataPost, User, Users},
    pagination::{Direction, SortBy},
    version::Version
};

pub type Pool = sqlx::Pool<Sqlite>;

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatabaseError(e.to_string())
    }
}

#[derive(Clone)]
pub struct SqlxDatabaseClient<DB: Database>(pub sqlx::Pool<DB>);

#[async_trait]
impl DatabaseClient for SqlxDatabaseClient<Sqlite> {
    async fn get_project_id(
        &self,
        project: &str
    ) -> Result<ProjectID, AppError>
    {
        get_project_id(&self.0, project).await
    }

    async fn get_project_count(
        &self,
    ) -> Result<i32, AppError>
    {
        get_project_count(&self.0).await
    }

    async fn get_user_id(
        &self,
        user: &str
    ) -> Result<i64, AppError>
    {
        get_user_id(&self.0, user).await
    }

    async fn get_owners(
        &self,
        proj_id: i64
    ) -> Result<Users, AppError>
    {
        get_owners(&self.0, proj_id).await
    }

    async fn user_is_owner(
        &self,
        user: &User,
        proj_id: i64
    ) -> Result<bool, AppError>
    {
        user_is_owner(&self.0, user, proj_id).await
    }

    async fn add_owner(
        &self,
        user_id: i64,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        add_owner(&self.0, user_id, proj_id).await
    }

    async fn add_owners(
        &self,
        owners: &Users,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        add_owners(&self.0, owners, proj_id).await
    }

    async fn remove_owner(
        &self,
        user_id: i64,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        remove_owner(&self.0, user_id, proj_id).await
    }

    async fn remove_owners(
        &self,
        owners: &Users,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        remove_owners(&self.0, owners, proj_id).await
    }

    async fn has_owner(
        &self,
        proj_id: i64,
    ) -> Result<bool, AppError>
    {
        has_owner(&self.0, proj_id).await
    }

    async fn get_projects_start_window(
        &self,
        query: Option<&str>,
        sort_by: SortBy,
        limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>
    {
        get_projects_start_window(&self.0, query, sort_by, limit).await
    }

    async fn get_projects_end_window(
        &self,
        query: Option<&str>,
        sort_by: SortBy,
        limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>
    {
        get_projects_end_window(&self.0, query, sort_by, limit).await
    }

    async fn get_projects_after_window(
        &self,
        query: Option<&str>,
        sort_by: SortBy,
        name: &str,
        id: u32,
        limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>
    {
        get_projects_after_window(&self.0, query, sort_by, name, id, limit).await
    }

    async fn get_projects_before_window(
        &self,
        query: Option<&str>,
        sort_by: SortBy,
        name: &str,
        id: u32,
        limit: u32
    ) -> Result<Vec<ProjectSummaryRow>, AppError>
    {
        get_projects_before_window(&self.0, query, sort_by, name, id, limit).await
    }

    async fn create_project(
        &self,
        user: &User,
        proj: &str,
        proj_data: &ProjectDataPost,
        now: &str
    ) -> Result<(), AppError>
    {
        create_project(&self.0, user, proj, proj_data, now).await
    }

    async fn update_project(
        &self,
        owner: &Owner,
        proj_id: i64,
        proj_data: &ProjectDataPatch,
        now: &str
    ) -> Result<(), AppError>
    {
        update_project(&self.0, owner, proj_id, proj_data, now).await
    }

    async fn get_project_row(
        &self,
        proj_id: i64
    ) -> Result<ProjectRow, AppError>
    {
        get_project_row(&self.0, proj_id).await
    }

    async fn get_project_row_revision(
        &self,
        proj_id: i64,
        revision: i64
    ) -> Result<ProjectRow, AppError>
    {
        get_project_row_revision(&self.0, proj_id, revision).await
    }

    async fn get_packages(
        &self,
        proj_id: i64
    ) -> Result<Vec<PackageRow>, AppError>
    {
        get_packages(&self.0, proj_id).await
    }

    async fn get_packages_at(
        &self,
        proj_id: i64,
        date: &str,
    ) -> Result<Vec<PackageRow>, AppError>
    {
        get_packages_at(&self.0, proj_id, date).await
    }

    async fn get_releases(
        &self,
        pkg_id: i64
    ) -> Result<Vec<ReleaseRow>, AppError>
    {
        get_releases(&self.0, pkg_id).await
    }

    async fn get_releases_at(
        &self,
        pkg_id: i64,
        date: &str
    ) -> Result<Vec<ReleaseRow>, AppError>
    {
        get_releases_at(&self.0, pkg_id, date).await
    }

    async fn get_authors(
        &self,
        pkg_ver_id: i64
    ) -> Result<Users, AppError>
    {
        get_authors(&self.0, pkg_ver_id).await
    }

    async fn get_package_url(
        &self,
        pkg_id: i64
    ) -> Result<String, AppError>
    {
        get_package_url(&self.0, pkg_id).await
    }

     async fn get_release_url(
        &self,
        pkg_id: i64,
        version: &Version
    ) -> Result<String, AppError>
    {
        get_release_url(&self.0, pkg_id, version).await
    }

    async fn get_players(
        &self,
        proj_id: i64
    ) -> Result<Users, AppError>
    {
        get_players(&self.0, proj_id).await
    }

    async fn add_player(
        &self,
        player: &User,
        proj_id: i64,
    ) -> Result<(), AppError>
    {
        add_player(&self.0, player, proj_id).await
    }

    async fn remove_player(
        &self,
        player: &User,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        remove_player(&self.0, player, proj_id).await
    }

    async fn get_image_url(
        &self,
        proj_id: i64,
        img_name: &str
    ) -> Result<String, AppError>
    {
        get_image_url(&self.0, proj_id, img_name).await
    }
}

async fn get_project_id<'e, E>(
    ex: E,
    project: &str
) -> Result<ProjectID, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query_scalar!(
        "
SELECT project_id
FROM projects
WHERE name = ?
        ",
        project
    )
    .fetch_optional(ex)
    .await?
    .map(ProjectID)
    .ok_or(AppError::NotAProject)
}

async fn get_project_count<'e, E>(
    ex: E
) -> Result<i32, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query_scalar!(
            "
SELECT COUNT(1)
FROM projects
            "
        )
        .fetch_one(ex)
        .await?
    )
}

async fn get_user_id<'e, E>(
    ex: E,
    user: &str
) -> Result<i64, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query_scalar!(
        "
SELECT user_id
FROM users
WHERE username = ?
LIMIT 1
        ",
        user
    )
    .fetch_optional(ex)
    .await?
    .ok_or(AppError::NotAUser)
}

async fn get_owners<'e, E>(
    ex: E,
    proj_id: i64
) -> Result<Users, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        Users {
            users: sqlx::query_scalar!(
                "
SELECT users.username
FROM users
JOIN owners
ON users.user_id = owners.user_id
JOIN projects
ON owners.project_id = projects.project_id
WHERE projects.project_id = ?
ORDER BY users.username
                ",
                proj_id
            )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(User)
            .collect()
        }
    )
}

async fn user_is_owner<'e, E>(
    ex: E,
    user: &User,
    proj_id: i64
) -> Result<bool, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query!(
            "
SELECT 1 AS present
FROM owners
JOIN users
ON users.user_id = owners.user_id
WHERE users.username = ? AND owners.project_id = ?
LIMIT 1
            ",
            user.0,
            proj_id
        )
        .fetch_optional(ex)
        .await?
        .is_some()
    )
}

async fn add_owner<'e, E>(
    ex: E,
    user_id: i64,
    proj_id: i64
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query!(
        "
INSERT OR IGNORE INTO owners (
    user_id,
    project_id
)
VALUES (?, ?)
        ",
        user_id,
        proj_id
    )
    .execute(ex)
    .await?;

    Ok(())
}

async fn add_owners<'a, A>(
    conn: A,
    owners: &Users,
    proj_id: i64
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    for owner in &owners.users {
        // get user id of new owner
        let owner_id = get_user_id(&mut *tx, &owner.0).await?;
        // associate new owner with the project
        add_owner(&mut *tx, owner_id, proj_id).await?;
    }

    tx.commit().await?;

    Ok(())
}

async fn remove_owner<'e, E>(
    ex: E,
    user_id: i64,
    proj_id: i64
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query!(
        "
DELETE FROM owners
WHERE user_id = ?
    AND project_id = ?
        ",
        user_id,
        proj_id
    )
    .execute(ex)
    .await?;

    Ok(())
}

async fn remove_owners<'a, A>(
    conn: A,
    owners: &Users,
    proj_id: i64
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    for owner in &owners.users {
        // get user id of owner
        let owner_id = get_user_id(&mut *tx, &owner.0).await?;
        // remove old owner from the project
        remove_owner(&mut *tx, owner_id, proj_id).await?;
    }

    // prevent removal of last owner
    if !has_owner(&mut *tx, proj_id).await? {
        return Err(AppError::CannotRemoveLastOwner);
    }

    tx.commit().await?;

    Ok(())
}

async fn has_owner<'e, E>(
    ex: E,
    proj_id: i64,
) -> Result<bool, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query!(
            "
SELECT 1 AS present
FROM owners
WHERE project_id = ?
LIMIT 1
            ",
            proj_id
        )
        .fetch_optional(ex)
        .await?
        .is_some()
    )
}

impl SortBy {
    fn field(&self) -> &'static str {
        match self {
            SortBy::ProjectName => "name",
            SortBy::GameTitle => "game_title_sort",
            SortBy::ModificationTime => "modified_at",
            SortBy::CreationTime => "created_at"
        }
    }
}

impl Direction {
    fn dir(&self) -> &'static str {
        match self {
            Direction::Ascending => "ASC",
            Direction::Descending => "DESC"
        }
    }

    fn op(&self) -> &'static str {
        match self {
            Direction::Ascending => ">",
            Direction::Descending => "<"
        }
    }
}

async fn get_projects_window_start_end_impl<'e, E>(
    ex: E,
    sort_by: SortBy,
    dir: Direction,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        QueryBuilder::new(
            "
SELECT
    project_id,
    name,
    description,
    revision,
    created_at,
    modified_at,
    game_title,
    game_title_sort,
    game_publisher,
    game_year,
    image
FROM projects
ORDER BY "
        )
        .push(sort_by.field())
        .push(" COLLATE NOCASE ")
        .push(dir.dir())
        .push(", project_id ")
        .push(dir.dir())
        .push(" LIMIT ")
        .push_bind(limit)
        .build_query_as::<ProjectSummaryRow>()
        .fetch_all(ex)
        .await?
    )
}

async fn get_projects_window_start_end_query_impl<'e, E>(
    ex: E,
    query: &str,
    sort_by: SortBy,
    dir: Direction,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        QueryBuilder::new(
            "
SELECT
    projects.project_id,
    projects.name,
    projects.description,
    projects.revision,
    projects.created_at,
    projects.modified_at,
    projects.game_title,
    projects.game_title_sort,
    projects.game_publisher,
    projects.game_year,
    projects.image
FROM projects
JOIN projects_fts
ON projects.project_id = projects_fts.rowid
WHERE projects_fts MATCH "
        )
        .push_bind(query)
        .push(" ORDER BY projects_fts.rank ")
        .push(dir.dir())
        .push(", projects.project_id ")
        .push(dir.dir())
        .push(" LIMIT ")
        .push_bind(limit)
        .build_query_as::<ProjectSummaryRow>()
        .fetch_all(ex)
        .await?
    )
}

async fn get_projects_window_before_after_impl<'e, E>(
    ex: E,
    sort_by: SortBy,
    dir: Direction,
    name: &str,
    id: u32,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        QueryBuilder::new(
            "
SELECT
    project_id,
    name,
    description,
    revision,
    created_at,
    modified_at,
    game_title,
    game_title_sort,
    game_publisher,
    game_year,
    image
FROM projects
WHERE "
        )
        .push(sort_by.field())
        .push(" ")
        .push(dir.op())
        .push(" ")
        .push_bind(name)
        .push(" OR (")
        .push(sort_by.field())
        .push(" = ")
        .push_bind(name)
        .push(" AND project_id ")
        .push(dir.op())
        .push(" ")
        .push_bind(id)
        .push(") ORDER BY ")
        .push(sort_by.field())
        .push(" COLLATE NOCASE ")
        .push(dir.dir())
        .push(", project_id ")
        .push(dir.dir())
        .push(" LIMIT ")
        .push_bind(limit)
        .build_query_as::<ProjectSummaryRow>()
        .fetch_all(ex)
        .await?
    )
}

async fn get_projects_start_window<'e, E>(
    ex: E,
    query: Option<&str>,
    sort_by: SortBy,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    match query {
        Some(q) => get_projects_window_start_end_query_impl(
            ex,
            q,
            sort_by,
            Direction::Ascending,
            limit
        ).await,
        None => get_projects_window_start_end_impl(
            ex,
            sort_by,
            Direction::Ascending,
            limit
        ).await
    }
}

async fn get_projects_end_window<'e, E>(
    ex: E,
    query: Option<&str>,
    sort_by: SortBy,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    // end is just start in the other direction
    get_projects_window_start_end_impl(
        ex,
        sort_by,
        Direction::Descending,
        limit
    ).await
}

async fn get_projects_after_window<'e, E>(
    ex: E,
    query: Option<&str>,
    sort_by: SortBy,
    name: &str,
    id: u32,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    get_projects_window_before_after_impl(
        ex,
        sort_by,
        Direction::Ascending,
        name,
        id,
        limit
    ).await
}

async fn get_projects_before_window<'e, E>(
    ex: E,
    query: Option<&str>,
    sort_by: SortBy,
    name: &str,
    id: u32,
    limit: u32
) -> Result<Vec<ProjectSummaryRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    get_projects_window_before_after_impl(
        ex,
        sort_by,
        Direction::Descending,
        name,
        id,
        limit
    ).await
}

async fn create_project_row<'e, E>(
    ex: E,
    user_id: i64,
    proj: &str,
    proj_data: &ProjectDataPost,
    now: &str
) -> Result<i64, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query_scalar!(
            "
INSERT INTO projects (
    name,
    created_at,
    description,
    game_title,
    game_title_sort,
    game_publisher,
    game_year,
    readme,
    image,
    modified_at,
    modified_by,
    revision
)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
RETURNING project_id
            ",
            proj,
            now,
            proj_data.description,
            proj_data.game.title,
            proj_data.game.title_sort_key,
            proj_data.game.publisher,
            proj_data.game.year,
            "",
            None::<&str>,
            now,
            user_id,
            1
        )
        .fetch_one(ex)
        .await?
    )
}

#[derive(Debug)]
struct ProjectDataRow<'a> {
    project_id: i64,
    description: &'a str,
    game_title: &'a str,
    game_title_sort: &'a str,
    game_publisher: &'a str,
    game_year: &'a str,
    readme: &'a str,
    image: Option<&'a str>
}

async fn create_project_data_row<'e, E>(
    ex: E,
    row: &ProjectDataRow<'_>
) -> Result<i64, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query_scalar!(
            "
INSERT INTO project_data (
    project_id,
    description,
    game_title,
    game_title_sort,
    game_publisher,
    game_year,
    readme,
    image
)
VALUES (?, ?, ?, ?, ?, ?, ?, ?)
RETURNING project_data_id
            ",
            row.project_id,
            row.description,
            row.game_title,
            row.game_title_sort,
            row.game_publisher,
            row.game_year,
            row.readme,
            row.image
        )
        .fetch_one(ex)
        .await?
    )
}

#[derive(Debug)]
struct ProjectRevisionRow<'a> {
    project_id: i64,
    name: &'a str,
    created_at: &'a str,
    modified_at: &'a str,
    modified_by: i64,
    revision: i64,
    project_data_id: i64
}

async fn create_project_revision_row<'e, E>(
    ex: E,
    row: &ProjectRevisionRow<'_>
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query!(
        "
INSERT INTO project_revisions (
    project_id,
    name,
    created_at,
    modified_at,
    modified_by,
    revision,
    project_data_id
)
VALUES (?, ?, ?, ?, ?, ?, ?)
        ",
        row.project_id,
        row.name,
        row.created_at,
        row.modified_at,
        row.modified_by,
        row.revision,
        row.project_data_id
    )
    .execute(ex)
    .await?;

    Ok(())
}

async fn create_project<'a, A>(
    conn: A,
    user: &User,
    proj: &str,
    pd: &ProjectDataPost,
    now: &str
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    // get user id of new owner
    let owner_id = get_user_id(&mut *tx, &user.0).await?;

    // create project row
    let proj_id = create_project_row(&mut *tx, owner_id, proj, pd, now).await?;

    // associate new owner with the project
    add_owner(&mut *tx, owner_id, proj_id).await?;

    // create project revision
    let dr = ProjectDataRow {
        project_id: proj_id,
        description: &pd.description,
        game_title: &pd.game.title,
        game_title_sort: &pd.game.title_sort_key,
        game_publisher:  &pd.game.publisher,
        game_year: &pd.game.year,
        readme: &pd.readme,
        image: pd.image.as_deref()
    };

    let project_data_id = create_project_data_row(&mut *tx, &dr).await?;

    let rr = ProjectRevisionRow {
        project_id: proj_id,
        name: proj,
        created_at: now,
        modified_at: now,
        modified_by: owner_id,
        revision: 1,
        project_data_id
    };

    create_project_revision_row(&mut *tx, &rr).await?;

    tx.commit().await?;

    Ok(())
}

async fn update_project_row<'e, E>(
    ex: E,
    owner_id: i64,
    proj_id: i64,
    revision: i64,
    pd: &ProjectDataPatch,
    now: &str
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    let mut qb: QueryBuilder<E::Database> = QueryBuilder::new(
        "UPDATE projects SET "
    );

    let mut qbs = qb.separated(", ");

    qbs
        .push("revision = ")
        .push_bind_unseparated(revision)
        .push("modified_at = ")
        .push_bind_unseparated(now)
        .push("modified_by = ")
        .push_bind_unseparated(owner_id);

    if let Some(description) = &pd.description {
        qbs.push("description = ").push_bind_unseparated(description);
    }

    if let Some(game_title) = &pd.game.title {
        qbs.push("game_title = ").push_bind_unseparated(game_title);
    }

    if let Some(game_title_sort) = &pd.game.title_sort_key {
        qbs.push("game_title_sort = ").push_bind_unseparated(game_title_sort);
    }

    if let Some(game_publisher) = &pd.game.publisher {
        qbs.push("game_publisher = ").push_bind_unseparated(game_publisher);
    }

    if let Some(game_year) = &pd.game.year {
        qbs.push("game_year = ").push_bind_unseparated(game_year);
    }

    if let Some(readme) = &pd.readme {
        qbs.push("readme = ").push_bind_unseparated(readme);
    }

    if let Some(image) = &pd.image {
        qbs.push("image = ").push_bind_unseparated(image);
    }

    qb
        .push(" WHERE project_id = ")
        .push_bind(proj_id)
        .build()
        .execute(ex)
        .await?;

    Ok(())
}

// TODO: update project mtime when packages change

async fn update_project<'a, A>(
    conn: A,
    owner: &Owner,
    proj_id: i64,
    pd: &ProjectDataPatch,
    now: &str
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    // get user id of owner
    let owner_id = get_user_id(&mut *tx, &owner.0).await?;

    // get project
    let row = get_project_row(&mut *tx, proj_id).await?;
    let revision = row.revision + 1;

    // update project
    update_project_row(&mut *tx, owner_id, proj_id, revision, pd, now).await?;

    // create project revision
    let dr = ProjectDataRow {
        project_id: proj_id,
        description: pd.description.as_ref().unwrap_or(&row.description),
        game_title: pd.game.title.as_ref().unwrap_or(&row.game_title),
        game_title_sort: pd.game.title_sort_key.as_ref().unwrap_or(&row.game_title_sort),
        game_publisher: pd.game.publisher.as_ref().unwrap_or(&row.game_publisher),
        game_year: pd.game.year.as_ref().unwrap_or(&row.game_year),
        readme: pd.readme.as_ref().unwrap_or(&row.readme),
        image: pd.image.as_ref().unwrap_or(&row.image).as_deref()
    };

    let project_data_id = create_project_data_row(&mut *tx, &dr).await?;

    let rr = ProjectRevisionRow {
        project_id: proj_id,
        name: &row.name,
        created_at: &row.created_at,
        modified_at: now,
        modified_by: owner_id,
        revision,
        project_data_id
    };

    create_project_revision_row(&mut *tx, &rr).await?;

    tx.commit().await?;

    Ok(())
}

async fn get_project_row<'e, E>(
    ex: E,
    proj_id: i64
) -> Result<ProjectRow, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query_as!(
        ProjectRow,
        "
SELECT
    project_id,
    name,
    description,
    revision,
    created_at,
    modified_at,
    modified_by,
    game_title,
    game_title_sort,
    game_publisher,
    game_year,
    readme,
    image
FROM projects
WHERE project_id = ?
LIMIT 1
        ",
        proj_id
    )
    .fetch_optional(ex)
    .await?
    .ok_or(AppError::NotAProject)
}

async fn get_project_row_revision<'e, E>(
    ex: E,
    proj_id: i64,
    revision: i64
) -> Result<ProjectRow, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query_as!(
        ProjectRow,
        "
SELECT
    project_revisions.project_id,
    project_revisions.name,
    project_data.description,
    project_revisions.revision,
    project_revisions.created_at,
    project_revisions.modified_at,
    project_revisions.modified_by,
    project_data.game_title,
    project_data.game_title_sort,
    project_data.game_publisher,
    project_data.game_year,
    project_data.image,
    project_data.readme
FROM project_revisions
JOIN project_data
ON project_revisions.project_data_id = project_data.project_data_id
WHERE project_revisions.project_id = ?
    AND project_revisions.revision = ?
LIMIT 1
        ",
        proj_id,
        revision
    )
    .fetch_optional(ex)
    .await?
    .ok_or(AppError::NotARevision)
}

async fn get_packages<'e, E>(
    ex: E,
    proj_id: i64
) -> Result<Vec<PackageRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query_as!(
            PackageRow,
            "
SELECT
    package_id,
    name,
    created_at
FROM packages
WHERE project_id = ?
ORDER BY name COLLATE NOCASE ASC
            ",
            proj_id
        )
       .fetch_all(ex)
       .await?
    )
}

async fn get_packages_at<'e, E>(
    ex: E,
    proj_id: i64,
    date: &str
) -> Result<Vec<PackageRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        sqlx::query_as!(
            PackageRow,
            "
SELECT
    package_id,
    name,
    created_at
FROM packages
WHERE project_id = ?
    AND created_at <= ?
ORDER BY name COLLATE NOCASE ASC
            ",
            proj_id,
            date
        )
       .fetch_all(ex)
       .await?
    )
}

// TODO: can we combine these?
// TODO: make Version borrow Strings?
impl<'r> From<&'r ReleaseRow> for Version {
    fn from(r: &'r ReleaseRow) -> Self {
        Version {
            major: r.version_major,
            minor: r.version_minor,
            patch: r.version_patch,
            pre: Some(&r.version_pre).filter(|v| !v.is_empty()).cloned(),
            build: Some(&r.version_build).filter(|v| !v.is_empty()).cloned()
        }
    }
}

impl<'r> From<&'r ReducedReleaseRow> for Version {
    fn from(r: &'r ReducedReleaseRow) -> Self {
        Version {
            major: r.version_major,
            minor: r.version_minor,
            patch: r.version_patch,
            pre: Some(&r.version_pre).filter(|v| !v.is_empty()).cloned(),
            build: Some(&r.version_build).filter(|v| !v.is_empty()).cloned()
        }
    }
}

fn release_row_cmp<R>(a: &R, b: &R) -> Ordering
where
    Version: for<'r> From<&'r R>
{
    let av: Version = a.into();
    let bv = b.into();
    av.cmp(&bv)
}

async fn get_releases<'e, E>(
    ex: E,
    pkg_id: i64
) -> Result<Vec<ReleaseRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    let mut releases = sqlx::query_as!(
        ReleaseRow,
        "
SELECT
    releases.release_id,
    releases.version,
    version_major,
    version_minor,
    version_patch,
    version_pre,
    version_build,
    releases.url,
    releases.filename,
    releases.size,
    releases.checksum,
    releases.published_at,
    users.username AS published_by
FROM releases
JOIN users
ON releases.published_by = users.user_id
WHERE package_id = ?
ORDER BY
    version_major DESC,
    version_minor DESC,
    version_patch DESC,
    version_pre ASC,
    version_build ASC
        ",
        pkg_id
    )
    .fetch_all(ex)
    .await?;

    releases.sort_by(|a, b| release_row_cmp(b, a));
    Ok(releases)
}

async fn get_releases_at<'e, E>(
    ex: E,
    pkg_id: i64,
    date: &str
) -> Result<Vec<ReleaseRow>, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    let mut releases = sqlx::query_as!(
        ReleaseRow,
        "
SELECT
    releases.release_id,
    releases.version,
    version_major,
    version_minor,
    version_patch,
    version_pre,
    version_build,
    releases.url,
    releases.filename,
    releases.size,
    releases.checksum,
    releases.published_at,
    users.username AS published_by
FROM releases
JOIN users
ON releases.published_by = users.user_id
WHERE package_id = ?
    AND published_at <= ?
ORDER BY
    version_major DESC,
    version_minor DESC,
    version_patch DESC,
    version_pre ASC,
    version_build ASC
        ",
        pkg_id,
        date
    )
    .fetch_all(ex)
    .await?;

    releases.sort_by(|a, b| release_row_cmp(b, a));
    Ok(releases)
}

async fn get_authors<'e, E>(
    ex: E,
    pkg_ver_id: i64
) -> Result<Users, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        Users {
            users: sqlx::query_scalar!(
                "
SELECT users.username
FROM users
JOIN authors
ON users.user_id = authors.user_id
WHERE authors.release_id = ?
ORDER BY users.username
                ",
                pkg_ver_id
            )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(User)
            .collect()
        }
    )
}

#[derive(Debug, Deserialize)]
struct ReducedReleaseRow {
    url: String,
    version_major: i64,
    version_minor: i64,
    version_patch: i64,
    version_pre: String,
    version_build: String,
}

// TODO: figure out how to order version_pre
async fn get_package_url<'e, E>(
    ex: E,
    pkg_id: i64
) -> Result<String, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    let mut releases = sqlx::query_as!(
        ReducedReleaseRow,
        "
SELECT
    url,
    version_major,
    version_minor,
    version_patch,
    version_pre,
    version_build
FROM releases
WHERE package_id = ?
ORDER BY
    version_major DESC,
    version_minor DESC,
    version_patch DESC,
    version_pre ASC,
    version_build ASC
        ",
        pkg_id
    )
    .fetch_all(ex)
    .await?;

    match releases.is_empty() {
        true => Err(AppError::NotAPackage),
        false => {
            releases.sort_by(release_row_cmp);
            Ok(releases.pop().unwrap().url)
        }
    }
}

async fn get_release_url<'e, E>(
    ex: E,
    pkg_id: i64,
    version: &Version
) -> Result<String, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    let pre = version.pre.as_deref().unwrap_or("");
    let build = version.build.as_deref().unwrap_or("");

    sqlx::query_scalar!(
        "
SELECT url
FROM releases
WHERE package_id = ?
    AND version_major = ?
    AND version_minor = ?
    AND version_patch = ?
    AND version_pre = ?
    AND version_build = ?
LIMIT 1
        ",
        pkg_id,
        version.major,
        version.minor,
        version.patch,
        pre,
        build
    )
    .fetch_optional(ex)
    .await?
    .ok_or(AppError::NotAVersion)
}

async fn get_players<'e, E>(
    ex: E,
    proj_id: i64
) -> Result<Users, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    Ok(
        Users {
            users: sqlx::query_scalar!(
                "
SELECT users.username
FROM users
JOIN players
ON users.user_id = players.user_id
JOIN projects
ON players.project_id = projects.project_id
WHERE projects.project_id = ?
ORDER BY users.username
                ",
                proj_id
            )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(User)
            .collect()
        }
    )
}

async fn add_player_id<'e, E>(
    ex: E,
    user_id: i64,
    proj_id: i64,
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query!(
        "
INSERT OR IGNORE INTO players (
    user_id,
    project_id
)
VALUES (?, ?)
        ",
        user_id,
        proj_id
    )
    .execute(ex)
    .await?;

    Ok(())
}

async fn add_player<'a, A>(
    conn: A,
    player: &User,
    proj_id: i64
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    // get user id of new player
    let player_id = get_user_id(&mut *tx, &player.0).await?;
    // associate new player with the project
    add_player_id(&mut *tx, player_id, proj_id).await?;

    tx.commit().await?;

    Ok(())
}

async fn remove_player_id<'e, E>(
    ex: E,
    user_id: i64,
    proj_id: i64
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query!(
        "
DELETE FROM players
WHERE user_id = ?
    AND project_id = ?
        ",
        user_id,
        proj_id
    )
    .execute(ex)
    .await?;

    Ok(())
}

async fn remove_player<'a, A>(
    conn: A,
    player: &User,
    proj_id: i64
) -> Result<(), AppError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    // get user id of player
    let player_id = get_user_id(&mut *tx, &player.0).await?;
    // remove player from the project
    remove_player_id(&mut *tx, player_id, proj_id).await?;

    tx.commit().await?;

    Ok(())
}

async fn get_image_url<'e, E>(
    ex: E,
    proj_id: i64,
    img_name: &str
) -> Result<String, AppError>
where
    E: Executor<'e, Database = Sqlite>
{
    sqlx::query_scalar!(
        "
SELECT url
FROM images
WHERE project_id = ?
    AND filename = ?
LIMIT 1
        ",
        proj_id,
        img_name
     )
     .fetch_optional(ex)
     .await?
     .ok_or(AppError::NotFound)
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::model::GameData;

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_id_ok(pool: Pool) {
        assert_eq!(
            get_project_id(&pool, "test_game").await.unwrap(),
            ProjectID(42)
        );
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_id_not_a_project(pool: Pool) {
        assert_eq!(
            get_project_id(&pool, "bogus").await.unwrap_err(),
            AppError::NotAProject
        );
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_count_ok(pool: Pool) {
        assert_eq!(get_project_count(&pool).await.unwrap(), 2);
    }

    #[sqlx::test(fixtures("users"))]
    async fn get_user_id_ok(pool: Pool) {
        assert_eq!(get_user_id(&pool, "bob").await.unwrap(), 1);
    }

    #[sqlx::test(fixtures("users"))]
    async fn get_user_id_not_a_user(pool: Pool) {
        assert_eq!(
            get_user_id(&pool, "not_a_user").await.unwrap_err(),
            AppError::NotAUser
        );
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn get_owners_ok(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
    }

// TODO: can we tell when the project doesn't exist?
/*
    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn get_owners_not_a_project(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 0).await.unwrap_err(),
            AppError::NotAProject
        );
    }
*/

// TODO: can we tell when the project doesn't exist?

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn user_is_owner_true(pool: Pool) {
        assert!(user_is_owner(&pool, &User("bob".into()), 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users", "projects","one_owner"))]
    async fn user_is_owner_false(pool: Pool) {
        assert!(!user_is_owner(&pool, &User("alice".into()), 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn add_owner_new(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
        add_owner(&pool, 2, 42).await.unwrap();
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn add_owner_existing(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
        add_owner(&pool, 1, 42).await.unwrap();
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
    }

// TODO: add test for add_owner not a project
// TODO: add test for add_owner not a user

    #[sqlx::test(fixtures("users", "projects","one_owner"))]
    async fn remove_owner_ok(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
        remove_owner(&pool, 1, 42).await.unwrap();
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![] }
        );
    }

    #[sqlx::test(fixtures( "users", "projects", "one_owner"))]
    async fn remove_owner_not_an_owner(pool: Pool) {
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
        remove_owner(&pool, 2, 42).await.unwrap();
        assert_eq!(
            get_owners(&pool, 42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
    }

// TODO: add test for remove_owner not a project

// TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn has_owner_yes(pool: Pool) {
        assert!(has_owner(&pool, 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn has_owner_no(pool: Pool) {
        assert!(!has_owner(&pool, 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users"))]
    async fn create_project_ok(pool: Pool) {
        let user = User("bob".into());
        let row = ProjectRow {
            project_id: 1,
            name: "test_game".into(),
            description: "Brian's Trademarked Game of Being a Test Case".into(),
            revision: 1,
            created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
            modified_at: "2023-11-12T15:50:06.419538067+00:00".into(),
            modified_by: 1,
            game_title: "A Game of Tests".into(),
            game_title_sort: "Game of Tests, A".into(),
            game_publisher: "Test Game Company".into(),
            game_year: "1979".into(),
            readme: "".into(),
            image: None
        };

        let cdata = ProjectDataPost {
            description: row.description.clone(),
            tags: vec![],
            game: GameData {
                title: row.game_title.clone(),
                title_sort_key: row.game_title_sort.clone(),
                publisher: row.game_publisher.clone(),
                year: row.game_year.clone()
            },
            readme: "".into(),
            image: None
        };

        assert_eq!(
            get_project_id(&pool, &row.name).await.unwrap_err(),
            AppError::NotAProject
        );

        create_project(
            &pool,
            &user,
            &row.name,
            &cdata,
            &row.created_at
        ).await.unwrap();

        let proj_id = get_project_id(&pool, &row.name).await.unwrap();

        assert_eq!(
            get_project_row(&pool, proj_id.0).await.unwrap(),
            row
        );
    }

// TODO: add tests for create_project
/*
    #[sqlx::test(fixtures("users"))]
    async fn create_project_bad(pool: Pool) {
    }
*/

// TODO: add tests for copy_project_revsion
// TODO: add tests for update_project

    fn fake_project_row(name: &str, id: usize) -> ProjectSummaryRow {
        ProjectSummaryRow {
            project_id: id as i64,
            name: name.into(),
            description: "".into(),
            revision: 1,
            created_at: "".into(),
            modified_at: "".into(),
            game_title: "".into(),
            game_title_sort: "".into(),
            game_publisher: "".into(),
            game_year: "".into(),
            image: None
        }
    }

    #[sqlx::test]
    async fn get_projects_start_window_empty(pool: Pool) {
        assert_eq!(
            get_projects_start_window(
                &pool, None, SortBy::ProjectName, 3
            ).await.unwrap(),
            []
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_start_window_not_all(pool: Pool) {
        assert_eq!(
            get_projects_start_window(
                &pool, None, SortBy::ProjectName, 3
            ).await.unwrap(),
            [
                fake_project_row("a", 1),
                fake_project_row("b", 2),
                fake_project_row("c", 3)
            ]
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_start_window_past_end(pool: Pool) {
        assert_eq!(
            get_projects_start_window(
                &pool, None, SortBy::ProjectName, 5 
            ).await.unwrap(),
            [
                fake_project_row("a", 1),
                fake_project_row("b", 2),
                fake_project_row("c", 3),
                fake_project_row("d", 4)
            ]
        );
    }

    #[sqlx::test]
    async fn get_projects_end_window_empty(pool: Pool) {
        assert_eq!(
            get_projects_end_window(
                &pool, None, SortBy::ProjectName, 3
            ).await.unwrap(),
            []
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_end_window_not_all(pool: Pool) {
        assert_eq!(
            get_projects_end_window(
                &pool, None, SortBy::ProjectName, 3
            ).await.unwrap(),
            [
                fake_project_row("d", 4),
                fake_project_row("c", 3),
                fake_project_row("b", 2),
            ]
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_end_window_past_start(pool: Pool) {
        assert_eq!(
            get_projects_end_window(
                &pool, None, SortBy::ProjectName, 5
            ).await.unwrap(),
            [
                fake_project_row("d", 4),
                fake_project_row("c", 3),
                fake_project_row("b", 2),
                fake_project_row("a", 1)
            ]
        );
    }

    #[sqlx::test]
    async fn get_projects_after_window_empty(pool: Pool) {
        assert_eq!(
            get_projects_after_window(
                &pool, None, SortBy::ProjectName, "a", 1, 3
            ).await.unwrap(),
            []
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_after_window_not_all(pool: Pool) {
        assert_eq!(
            get_projects_after_window(
                &pool, None, SortBy::ProjectName, "b", 2, 3
            ).await.unwrap(),
            [
                fake_project_row("c", 3),
                fake_project_row("d", 4)
            ]
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_after_window_past_end(pool: Pool) {
        assert_eq!(
            get_projects_after_window(
                &pool, None, SortBy::ProjectName, "d", 4, 3
            ).await.unwrap(),
            []
        );
    }

    #[sqlx::test]
    async fn get_projects_before_window_empty(pool: Pool) {
        assert_eq!(
            get_projects_before_window(
                &pool, None, SortBy::ProjectName, "a", 1, 3
            ).await.unwrap(),
            []
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_before_window_not_all(pool: Pool) {
        assert_eq!(
            get_projects_before_window(
                &pool, None, SortBy::ProjectName, "b", 2, 3
            ).await.unwrap(),
            [
                fake_project_row("a", 1)
            ]
        );
    }

    #[sqlx::test(fixtures("users", "proj_window"))]
    async fn get_projects_before_window_past_start(pool: Pool) {
        assert_eq!(
            get_projects_before_window(
                &pool, None, SortBy::ProjectName, "d", 4, 3
            ).await.unwrap(),
            [
                fake_project_row("c", 3),
                fake_project_row("b", 2),
                fake_project_row("a", 1)
            ]
        );
    }

/*
    #[sqlx::test(fixtures("users", "title_window"))]
    async fn get_projects_start_window_by_title(pool: Pool) {
        assert_eq!(
            get_projects_start_window(
                &pool, None, SortBy::GameTitle, Direction::Ascending, 1
            ).await.unwrap(),
            vec![
                ProjectSummaryRow {
                    project_id: 1,
                    name: "a".into(),
                    description: "".into(),
                    revision: 1,
                    created_at: "".into(),
                    modified_at: "".into(),
                    game_title: "".into(),
                    game_title_sort: "a".into(),
                    game_publisher: "".into(),
                    game_year: "".into(),
                    image: None
                }
            ]
        );
    }

    #[sqlx::test(fixtures("users", "title_window"))]
    async fn get_projects_after_window_by_title(pool: Pool) {
        assert_eq!(
            get_projects_after_window(
                &pool, None, SortBy::GameTitle, Direction::Ascending, "a", 1, 1
            ).await.unwrap(),
            vec![
                ProjectSummaryRow {
                    project_id: 2,
                    name: "b".into(),
                    description: "".into(),
                    revision: 1,
                    created_at: "".into(),
                    modified_at: "".into(),
                    game_title: "".into(),
                    game_title_sort: "a".into(),
                    game_publisher: "".into(),
                    game_year: "".into(),
                    image: None
                }
            ]
        );
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_row_ok(pool: Pool) {
        assert_eq!(
            get_project_row(&pool, 42).await.unwrap(),
            ProjectRow {
                project_id: 42,
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 3,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-12-14T15:50:06.419538067+00:00".into(),
                modified_by: 1,
                game_title: "A Game of Tests".into(),
                game_title_sort: "Game of Tests, A".into(),
                game_publisher: "Test Game Company".into(),
                game_year: "1979".into(),
                readme: "".into(),
                image: None
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_row_not_a_project(pool: Pool) {
        assert_eq!(
            get_project_row(&pool, 0).await.unwrap_err(),
            AppError::NotAProject
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners"))]
    async fn get_project_row_revision_ok_current(pool: Pool) {
        assert_eq!(
            get_project_row_revision(&pool, 42, 3).await.unwrap(),
            ProjectRow {
                project_id: 42,
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 3,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-12-14T15:50:06.419538067+00:00".into(),
                modified_by: 1,
                game_title: "A Game of Tests".into(),
                game_title_sort: "Game of Tests, A".into(),
                game_publisher: "Test Game Company".into(),
                game_year: "1979".into(),
                readme: "".into(),
                image: None
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners"))]
    async fn get_project_revision_ok_old(pool: Pool) {
        assert_eq!(
            get_project_row_revision(&pool, 42, 1).await.unwrap(),
            ProjectRow {
                project_id: 42,
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 1,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_by: 1,
                game_title: "A Game of Tests".into(),
                game_title_sort: "Game of Tests, A".into(),
                game_publisher: "Test Game Company".into(),
                game_year: "1978".into(),
                readme: "".into(),
                image: None
            }
        );
    }

// TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_revision_not_a_project(pool: Pool) {
        assert_eq!(
            get_project_row_revision(&pool, 0, 2).await.unwrap_err(),
            AppError::NotARevision
        );
    }

    #[sqlx::test(fixtures("users", "projects"))]
    async fn get_project_revision_not_a_revision(pool: Pool) {
        assert_eq!(
            get_project_row_revision(&pool, 42, 0).await.unwrap_err(),
            AppError::NotARevision
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_ok(pool: Pool) {
        assert_eq!(
            get_packages(&pool, 42).await.unwrap(),
            vec![
                PackageRow {
                    package_id: 1,
                    name: "a_package".into(),
                    created_at: "2023-12-09T15:56:29.180282477+00:00".into()
                },
                PackageRow {
                    package_id: 2,
                    name: "b_package".into(),
                    created_at: "2022-11-06T15:56:29.180282477+00:00".into()
                },
                PackageRow {
                    package_id: 3,
                    name: "c_package".into(),
                    created_at: "2023-11-06T15:56:29.180282477+00:00".into()
                }
            ]
        );
    }

// TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_not_a_project(pool: Pool) {
        assert_eq!(
            get_packages(&pool, 0).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_none(pool: Pool) {
        let date = "1970-01-01T00:00:00.000000000+00:00";
        assert_eq!(
            get_packages_at(&pool, 42, date).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_some(pool: Pool) {
        let date = "2023-01-01T00:00:00.000000000+00:00";
        assert_eq!(
            get_packages_at(&pool, 42, date).await.unwrap(),
            vec![
                PackageRow {
                    package_id: 2,
                    name: "b_package".into(),
                    created_at: "2022-11-06T15:56:29.180282477+00:00".into()
                }
            ]
        );
    }

    // TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_not_a_project(pool: Pool) {
        let date = "2022-01-01T00:00:00.000000000+00:00";
        assert_eq!(
            get_packages_at(&pool, 0, date).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_package_url_ok(pool: Pool) {
        assert_eq!(
            get_package_url(&pool, 1).await.unwrap(),
            "https://example.com/a_package-1.2.4"
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_package_url_not_a_package(pool: Pool) {
        assert_eq!(
            get_package_url(&pool, 0).await.unwrap_err(),
            AppError::NotAPackage
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_releases_ok(pool: Pool) {
        assert_eq!(
            get_releases(&pool, 1).await.unwrap(),
            vec![
                ReleaseRow {
                    release_id: 2,
                    version: "1.2.4".into(),
                    version_major: 1,
                    version_minor: 2,
                    version_patch: 4,
                    version_pre: "".into(),
                    version_build: "".into(),
                    url: "https://example.com/a_package-1.2.4".into(),
                    filename: "a_package-1.2.4".into(),
                    size: 5678,
                    checksum: "79fdd8fe3128f818e446e919cce5dcfb81815f8f4341c53f4d6b58ded48cebf2".into(),
                    published_at: "2023-12-10T15:56:29.180282477+00:00".into(),
                    published_by: "alice".into()
                },
                ReleaseRow {
                    release_id: 1,
                    version: "1.2.3".into(),
                    version_major: 1,
                    version_minor: 2,
                    version_patch: 3,
                    version_pre: "".into(),
                    version_build: "".into(),
                    url: "https://example.com/a_package-1.2.3".into(),
                    filename: "a_package-1.2.3".into(),
                    size: 1234,
                    checksum: "c0e0fa7373a12b45a91e4f4d4e2e186442fc6ee9b346caa2fdc1c09026a2144a".into(),
                    published_at: "2023-12-09T15:56:29.180282477+00:00".into(),
                    published_by: "bob".into()
                }
            ]
        );
    }

// TODO: can we tell when the package doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_releases_not_a_package(pool: Pool) {
        assert_eq!(
            get_releases(&pool, 0).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages", "authors"))]
    async fn get_authors_ok(pool: Pool) {
        assert_eq!(
            get_authors(&pool, 2).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

// TODO: can we tell when the package version doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages", "authors"))]
    async fn get_authors_not_a_release(pool: Pool) {
        assert_eq!(
            get_authors(&pool, 0).await.unwrap(),
            Users { users: vec![] }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn get_players_ok(pool: Pool) {
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

// TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn get_players_not_a_project(pool: Pool) {
        assert_eq!(
            get_players(&pool, 0).await.unwrap(),
            Users { users: vec![] }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn add_player_new(pool: Pool) {
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
        add_player(&pool, &User("chuck".into()), 42).await.unwrap();
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                    User("chuck".into())
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn add_player_existing(pool: Pool) {
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
        add_player(&pool, &User("alice".into()), 42).await.unwrap();
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
    }

// TODO: add test for add_player not a project
// TODO: add test for add_player not a user

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn remove_player_existing(pool: Pool) {
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
        remove_player(&pool, &User("alice".into()), 42).await.unwrap();
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("bob".into()),
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn remove_player_not_a_player(pool: Pool) {
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
        remove_player(&pool, &User("chuck".into()), 42).await.unwrap();
        assert_eq!(
            get_players(&pool, 42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into()),
                ]
            }
        );
    }

// TODO: add test for remove_player not a project

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_url_ok(pool: Pool) {
        assert_eq!(
            get_image_url(&pool, 42, "img.png").await.unwrap(),
            "https://example.com/images/img.png"
        );
    }

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_url_not_a_project(pool: Pool) {
        assert_eq!(
            get_image_url(&pool, 1, "img.png").await.unwrap_err(),
            AppError::NotFound
        );
    }

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_url_not_an_image(pool: Pool) {
        assert_eq!(
            get_image_url(&pool, 42, "bogus").await.unwrap_err(),
            AppError::NotFound
        );
    }
}
