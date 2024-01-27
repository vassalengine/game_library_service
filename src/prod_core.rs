use axum::async_trait;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use std::future::Future;

use crate::{
    core::Core,
    db::{DatabaseClient, PackageRow, ProjectRow, ReleaseRow},
    errors::AppError,
    model::{GameData, Owner, PackageData, Project, ProjectData, ProjectDataPatch, ProjectDataPost, ProjectID, Projects, ProjectSummary, ReleaseData, User, Users},
    pagination::{Anchor, Limit, OrderDirection, SortBy, Pagination, Seek, SeekLink},
    params::{ProjectsParams, SortOrSeek},
    version::Version
};

#[derive(Clone)]
pub struct ProdCore<C: DatabaseClient> {
    pub db: C,
    pub now: fn() -> DateTime<Utc>
}

// TODO: switch proj_id to proj_name; then we will always know if the project
// exists because we have to look up the id

#[async_trait]
impl<C: DatabaseClient + Send + Sync> Core for ProdCore<C> {
    async fn get_project_id(
         &self,
        proj: &Project
    ) -> Result<ProjectID, AppError>
    {
        self.db.get_project_id(&proj.0).await
    }

    async fn get_owners(
        &self,
        proj_id: i64
    ) -> Result<Users, AppError>
    {
        self.db.get_owners(proj_id).await
    }

    async fn add_owners(
        &self,
        owners: &Users,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        self.db.add_owners(owners, proj_id).await
    }

    async fn remove_owners(
        &self,
        owners: &Users,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        self.db.remove_owners(owners, proj_id).await
    }

    async fn user_is_owner(
        &self,
        user: &User,
        proj_id: i64
    ) -> Result<bool, AppError>
    {
        self.db.user_is_owner(user, proj_id).await
    }

    async fn get_projects(
        &self,
        params: ProjectsParams
    ) -> Result<Projects, AppError>
    {
        let query = params.q;
        let from = params.from;
        let limit = params.limit;

        let (prev_page, next_page, projects) = self.get_projects_from(
            query, from, limit
        ).await?;

        Ok(
            Projects {
                projects,
                meta: Pagination {
                    prev_page,
                    next_page,
                    total: self.db.get_project_count().await?
                }
            },
        )
    }

    async fn get_project(
        &self,
        proj_id: i64
    ) -> Result<ProjectData, AppError>
    {
        self.get_project_impl(
            proj_id,
            self.db.get_project_row(proj_id).await?,
            self.db.get_packages(proj_id).await?,
            |pc, pkgid| pc.db.get_releases(pkgid)
        ).await
    }

// TODO: require project names to match [A-Za-z0-9][A-Za-z0-9_-]{,63}?
// TODO: maybe also compare case-insensitively and equate - and _?
// TODO: length limits on strings
// TODO: require package names to match [A-Za-z0-9][A-Za-z0-9_-]{,63}?
// TODO: packages might need display names?

    async fn create_project(
        &self,
        user: &User,
        proj: &str,
        proj_data: &ProjectDataPost
    ) -> Result<(), AppError>
    {
        let now = (self.now)().to_rfc3339();
// FIXME: generate a sort key?
//        let mut proj_data = proj_data;
//        proj_data.game.title_sort_key = title_sort_key(&proj_data.game.title);
        self.db.create_project(user, proj, proj_data, &now).await
    }

    async fn update_project(
        &self,
        owner: &Owner,
        proj_id: i64,
        proj_data: &ProjectDataPatch
    ) -> Result<(), AppError>
    {
        let now = (self.now)().to_rfc3339();
        self.db.update_project(owner, proj_id, proj_data, &now).await
    }

    async fn get_project_revision(
        &self,
        proj_id: i64,
        revision: i64
    ) -> Result<ProjectData, AppError>
    {
        let proj_row = self.db.get_project_row_revision(
            proj_id, revision
        ).await?;

        let mtime = proj_row.modified_at.clone();

        let package_rows = self.db.get_packages_at(
            proj_id, &mtime
        ).await?;

        self.get_project_impl(
            proj_id,
            proj_row,
            package_rows,
            |pc, pkgid| pc.db.get_releases_at(pkgid, &mtime)
        ).await
    }

    async fn get_release(
        &self,
        _proj_id: i64,
        pkg_id: i64
    ) -> Result<String, AppError>
    {
        self.db.get_package_url(pkg_id).await
    }

    async fn get_release_version(
        &self,
        _proj_id: i64,
        pkg_id: i64,
        version: &Version
    ) -> Result<String, AppError>
    {
        self.db.get_release_url(pkg_id, version).await
    }

    async fn get_players(
        &self,
        proj_id: i64
    ) -> Result<Users, AppError>
    {
        self.db.get_players(proj_id).await
    }

    async fn add_player(
        &self,
        player: &User,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        self.db.add_player(player, proj_id).await
    }

    async fn remove_player(
        &self,
        player: &User,
        proj_id: i64
    ) -> Result<(), AppError>
    {
        self.db.remove_player(player, proj_id).await
    }

    async fn get_image(
        &self,
        proj_id: i64,
        img_name: &str
    ) -> Result<String, AppError>
    {
        self.db.get_image_url(proj_id, img_name).await
    }
}

impl<C: DatabaseClient + Send + Sync> ProdCore<C>  {
    async fn make_version_data(
        &self,
        rr: ReleaseRow
    ) -> Result<ReleaseData, AppError>
    {
        let authors = self.db.get_authors(rr.release_id)
            .await?
            .users
            .into_iter()
            .map(|u| u.0)
            .collect();

        Ok(
            ReleaseData {
                version: rr.version,
                filename: rr.filename,
                url: rr.url,
                size: rr.size,
                checksum: rr.checksum,
                published_at: rr.published_at,
                published_by: rr.published_by,
                requires: "".into(),
                authors
            }
        )
    }

    async fn make_package_data<'s, F, R>(
        &'s self,
        pr: PackageRow,
        get_release_rows: &F
    ) -> Result<PackageData, AppError>
    where
        F: Fn(&'s Self, i64) -> R,
        R: Future<Output = Result<Vec<ReleaseRow>, AppError>>
    {
        let releases = try_join_all(
            get_release_rows(self, pr.package_id)
                .await?
                .into_iter()
                .map(|vr| self.make_version_data(vr))
        ).await?;

        Ok(
            PackageData {
                name: pr.name,
                description: "".into(),
                releases
            }
        )
    }

    async fn get_project_impl<'s, F, R>(
        &'s self,
        proj_id: i64,
        proj_row: ProjectRow,
        package_rows: Vec<PackageRow>,
        get_release_rows: F
    ) -> Result<ProjectData, AppError>
    where
        F: Fn(&'s Self, i64) -> R,
        R: Future<Output = Result<Vec<ReleaseRow>, AppError>>
    {
        let owners = self.get_owners(proj_id)
            .await?
            .users
            .into_iter()
            .map(|u| u.0)
            .collect();

        let packages = try_join_all(
            package_rows
                .into_iter()
                .map(|pr| self.make_package_data(pr, &get_release_rows))
        ).await?;

        Ok(
            ProjectData {
                name: proj_row.name,
                description: proj_row.description,
                revision: proj_row.revision,
                created_at: proj_row.created_at,
                modified_at: proj_row.modified_at,
                tags: vec![],
                game: GameData {
                    title: proj_row.game_title,
                    title_sort_key: proj_row.game_title_sort,
                    publisher: proj_row.game_publisher,
                    year: proj_row.game_year
                },
                readme: proj_row.readme,
                image: proj_row.image,
                owners,
                packages
            }
        )
    }

    async fn get_projects_from(
        &self,
        query: Option<String>,
        from: SortOrSeek,
        limit: Limit
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        let limit = limit.get() as u32;

        match from {
            SortOrSeek::Sort(sort, dir) => self.get_projects_sort(query, sort, dir, limit).await,
            SortOrSeek::Seek(seek) => self.get_projects_seek(query, seek, limit).await
        }
    }

    async fn get_projects_sort(
        &self,
        query: Option<String>,
        sort: SortBy,
        dir: OrderDirection,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
//        Ok((None, None, vec![]))
        match dir {
            OrderDirection::Ascending => self.get_projects_start(query, &sort, limit).await,
            OrderDirection::Descending => self.get_projects_end(query, &sort, limit).await
        }
    }

    async fn get_projects_seek(
        &self,
        query: Option<String>,
        seek: Seek,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        match seek.anchor {
            Anchor::Start => self.get_projects_start(query, &seek.sort_by, limit).await,
            Anchor::After(id, name) => self.get_projects_after(query, &seek.sort_by, &name, id, limit).await,
            Anchor::Before(id, name) => self.get_projects_before(query, &seek.sort_by, &name, id, limit).await,
            Anchor::End => self.get_projects_end(query, &seek.sort_by, limit).await
        }
    }

    async fn get_projects_start(
        &self,
        query: Option<String>,
        sort_by: &SortBy,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        // try to get one extra so we can tell if we're at an endpoint
        let limit_extra = limit + 1;

        let mut projects = self.db.get_projects_start_window(
            query, sort_by, limit_extra
        ).await?;

        Ok(
            match projects.len() {
                l if l == limit_extra as usize => {
                    projects.pop();
                    let aid = projects[projects.len() - 1].project_id as u32;
                    let aname = projects[projects.len() - 1].name.clone();
                    (
                        None,
                        Some(
                            SeekLink::new(
                                Seek {
                                    anchor: Anchor::After(aid, aname),
                                    sort_by: SortBy::ProjectName
                                }
                            )
                        ),
                        projects.into_iter().map(ProjectSummary::from).collect()
                    )
                }
                _ => {
                    (
                        None,
                        None,
                        projects.into_iter().map(ProjectSummary::from).collect()
                    )
                }
            }
        )
    }

    async fn get_projects_end(
        &self,
        query: Option<String>,
        sort_by: &SortBy,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        // try to get one extra so we can tell if we're at an endpoint
        let limit_extra = limit + 1;

        let mut projects = self.db.get_projects_end_window(
            query, sort_by, limit_extra
        ).await?;

        Ok(
            if projects.len() == limit_extra as usize {
                projects.pop();
                projects.reverse();
                let bid = projects[0].project_id as u32;
                let bname = projects[0].name.clone();
                (
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::Before(bid, bname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    None,
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
            else {
                projects.reverse();
                (
                    None,
                    None,
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
        )
    }

    async fn get_projects_after(
        &self,
        query: Option<String>,
        sort_by: &SortBy,
        name: &str,
        id: u32,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        // try to get one extra so we can tell if we're at an endpoint
        let limit_extra = limit + 1;

        let mut projects = self.db.get_projects_after_window(
            query, sort_by, name, id, limit_extra
        ).await?;

        Ok(
            if projects.len() == limit_extra as usize {
                projects.pop();
                let bid = projects[0].project_id as u32;
                let bname = projects[0].name.clone();
                let aid = projects[projects.len() - 1].project_id as u32;
                let aname = projects[projects.len() - 1].name.clone();
                (
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::Before(bid, bname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::After(aid, aname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
            else if projects.is_empty() {
                (
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::End,
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    None,
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
            else {
                let bid = projects[0].project_id as u32;
                let bname = projects[0].name.clone();
                (
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::Before(bid, bname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    None,
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
        )
    }

    async fn get_projects_before(
        &self,
        query: Option<String>,
        sort_by: &SortBy,
        name: &str,
        id: u32,
        limit: u32
    ) -> Result<(Option<SeekLink>, Option<SeekLink>, Vec<ProjectSummary>), AppError>
    {
        // try to get one extra so we can tell if we're at an endpoint
        let limit_extra = limit + 1;

        let mut projects = self.db.get_projects_before_window(
            query, sort_by, name, id, limit_extra
        ).await?;

        Ok(
            if projects.len() == limit_extra as usize {
                projects.pop();
                projects.reverse();
                let bid = projects[0].project_id as u32;
                let bname = projects[0].name.clone();
                let aid = projects[projects.len() - 1].project_id as u32;
                let aname = projects[projects.len() - 1].name.clone();
                (
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::Before(bid, bname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::After(aid, aname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
            else if projects.is_empty() {
                (
                    None,
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::Start,
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
            else {
                projects.reverse();
                let aid = projects[projects.len() - 1].project_id as u32;
                let aname = projects[projects.len() - 1].name.clone();
                (
                    None,
                    Some(
                        SeekLink::new(
                            Seek {
                                anchor: Anchor::After(aid, aname),
                                sort_by: SortBy::ProjectName
                            }
                        )
                    ),
                    projects.into_iter().map(ProjectSummary::from).collect()
                )
            }
        )
    }
}

fn split_title_sort_key(title: &str) -> (&str, Option<&str>) {
    match title.split_once(' ') {
        // Probably Spanish or French, "A" is not an article
        Some(("A", rest)) if rest.starts_with("la") => (title, None),
        // Put leading article at end
        Some(("A", rest)) => (rest, Some("A")),
        Some(("An", rest)) => (rest, Some("An")),
        Some(("The", rest)) => (rest, Some("The")),
        // Doesn't start with an article
        Some(_) | None => (title, None)
    }
}

fn title_sort_key(title: &str) -> String {
    match split_title_sort_key(title) {
        (_, None) => title.into(),
        (rest, Some(art)) => format!("{rest}, {art}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use once_cell::sync::Lazy;

    use crate::{
        model::GameDataPatch,
        sqlite::{Pool, SqlxDatabaseClient}
    };

    const NOW: &str = "2023-11-12T15:50:06.419538067+00:00";

    static NOW_DT: Lazy<DateTime<Utc>> = Lazy::new(|| {
        DateTime::parse_from_rfc3339(NOW)
            .unwrap()
            .with_timezone(&Utc)
    });

    fn fake_now() -> DateTime<Utc> {
        *NOW_DT
    }

    fn make_core(
        pool: Pool,
        now: fn() -> DateTime<Utc>
    ) -> ProdCore<SqlxDatabaseClient<sqlx::sqlite::Sqlite>>
    {
        ProdCore {
            db: SqlxDatabaseClient(pool),
            now
        }
    }

    fn fake_project_summary(name: String) -> ProjectSummary {
        ProjectSummary {
            name,
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
    }

    #[sqlx::test(fixtures("users", "ten_projects"))]
    async fn get_projects_start_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let projects: Vec<ProjectSummary> = "abcde".chars()
            .map(|c| fake_project_summary(c.into()))
            .collect();

        let prev_page = None;
        let next_page = Some(
            SeekLink::new(
                Seek {
                    anchor: Anchor::After(5, "e".into()),
                    sort_by: SortBy::ProjectName
                }
            )
        );

        let params = ProjectsParams {
            from: SortOrSeek::Seek(
                Seek {
                    anchor: Anchor::Start,
                    sort_by: SortBy::ProjectName
                }
            ),
            limit: Limit::new(5).unwrap(),
            ..Default::default()
        };

        assert_eq!(
            core.get_projects(params).await.unwrap(),
            Projects {
                projects,
                meta: Pagination {
                    prev_page,
                    next_page,
                    total: 10
                }
            }
        );
    }

    #[sqlx::test(fixtures("users", "ten_projects"))]
    async fn get_projects_after_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let all_projects: Vec<ProjectSummary> = "abcdefghij".chars()
            .map(|c| fake_project_summary(c.into()))
            .collect();

        let lim = 5;

        // walk the limit window across the projects
        for i in 0..all_projects.len() {
            let projects: Vec<ProjectSummary> = all_projects.iter()
                .skip(i + 1)
                .take(lim)
                .cloned()
                .collect();

            let prev_page = if i == all_projects.len() - 1 {
                Some(
                    SeekLink::new(
                        Seek {
                            anchor: Anchor::End,
                            sort_by: SortBy::ProjectName
                        }
                    )
                )
            }
            else {
                projects
                    .first()
                    .map(|p| SeekLink::new(
                        Seek {
                            anchor: Anchor::Before(
                                (i + 2) as u32,
                                p.name.clone()
                            ),
                            sort_by: SortBy::ProjectName
                        }
                    ))
            };

            let next_page = if i + lim + 1 >= all_projects.len() {
                None
            }
            else {
                projects
                    .last()
                    .map(|p| SeekLink::new(
                        Seek {
                            anchor: Anchor::After(
                                (i + lim + 1) as u32,
                                p.name.clone()
                            ),
                            sort_by: SortBy::ProjectName
                        }
                    ))
            };

            let params = ProjectsParams {
                from: SortOrSeek::Seek(
                    Seek {
                        anchor: Anchor::After(
                            (i + 1) as u32,
                            all_projects[i].name.clone()
                        ),
                        sort_by: SortBy::ProjectName
                    }
                ),
                limit: Limit::new(lim as u8).unwrap(),
                ..Default::default()
            };

            assert_eq!(
                core.get_projects(params).await.unwrap(),
                Projects {
                    projects,
                    meta: Pagination {
                        prev_page,
                        next_page,
                        total: 10
                    }
                }
            );
        }
    }

    #[sqlx::test(fixtures("users", "ten_projects"))]
    async fn get_projects_before_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let all_projects: Vec<ProjectSummary> = "abcdefghij".chars()
            .map(|c| fake_project_summary(c.into()))
            .collect();

        let lim = 5;

        // walk the limit window across the projects
        for i in 0..all_projects.len() {
            let projects: Vec<ProjectSummary> = all_projects.iter()
                .skip(i.saturating_sub(lim))
                .take(i - i.saturating_sub(lim))
                .cloned()
                .collect();

            let prev_page = if i < lim + 1 {
                None
            }
            else {
                projects
                    .first()
                    .map(|p| SeekLink::new(
                        Seek {
                            anchor: Anchor::Before(
                                (i.saturating_sub(lim) + 1) as u32,
                                p.name.clone()
                            ),
                            sort_by: SortBy::ProjectName
                        }
                    ))
            };

            let next_page = if i == 0 {
                Some(
                    SeekLink::new(
                        Seek {
                            anchor: Anchor::Start,
                            sort_by: SortBy::ProjectName
                        }
                    )
                )
            }
            else {
                projects
                    .last()
                    .map(|p| SeekLink::new(
                        Seek {
                            anchor: Anchor::After(
                                i as u32,
                                p.name.clone()
                            ),
                            sort_by: SortBy::ProjectName
                        }
                    )
                )
            };

            let params = ProjectsParams {
                from: SortOrSeek::Seek(
                    Seek {
                        anchor: Anchor::Before(
                            (i + 1) as u32,
                            all_projects[i].name.clone()
                        ),
                        sort_by: SortBy::ProjectName
                    }
                ),
                limit: Limit::new(lim as u8).unwrap(),
                ..Default::default()
            };

            assert_eq!(
                core.get_projects(params).await.unwrap(),
                Projects {
                    projects,
                    meta: Pagination {
                        prev_page,
                        next_page,
                        total: 10
                    }
                }
            );
        }
    }

    #[sqlx::test(fixtures("users", "ten_projects"))]
    async fn get_projects_end_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let projects: Vec<ProjectSummary> = "fghij".chars()
            .map(|c| fake_project_summary(c.into()))
            .collect();

        let prev_page = Some(
            SeekLink::new(
                Seek {
                    anchor: Anchor::Before(6, "f".into()),
                    sort_by: SortBy::ProjectName
                }
            )
        );
        let next_page = None;

        let params = ProjectsParams {
            from: SortOrSeek::Seek(
                Seek {
                    anchor: Anchor::End,
                    sort_by: SortBy::ProjectName
                }
            ),
            limit: Limit::new(5).unwrap(),
            ..Default::default()
        };

        assert_eq!(
            core.get_projects(params).await.unwrap(),
            Projects {
                projects,
                 meta: Pagination {
                    prev_page,
                    next_page,
                    total: 10
                }
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners", "packages", "authors"))]
    async fn get_project_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_project(42).await.unwrap(),
            ProjectData {
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 3,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-12-14T15:50:06.419538067+00:00".into(),
                tags: vec![],
                game: GameData {
                    title: "A Game of Tests".into(),
                    title_sort_key: "Game of Tests, A".into(),
                    publisher: "Test Game Company".into(),
                    year: "1979".into()
                },
                readme: "".into(),
                image: None,
                owners: vec!["alice".into(), "bob".into()],
                packages: vec![
                    PackageData {
                        name: "a_package".into(),
                        description: "".into(),
                        releases: vec![
                            ReleaseData {
                                version: "1.2.4".into(),
                                filename: "a_package-1.2.4".into(),
                                url: "https://example.com/a_package-1.2.4".into(),
                                size: 5678,
                                checksum: "79fdd8fe3128f818e446e919cce5dcfb81815f8f4341c53f4d6b58ded48cebf2".into(),
                                published_at: "2023-12-10T15:56:29.180282477+00:00".into(),
                                published_by: "alice".into(),
                                requires: "".into(),
                                authors: vec!["alice".into(), "bob".into()]
                            },
                            ReleaseData {
                                version: "1.2.3".into(),
                                filename: "a_package-1.2.3".into(),
                                url: "https://example.com/a_package-1.2.3".into(),
                                size: 1234,
                                checksum: "c0e0fa7373a12b45a91e4f4d4e2e186442fc6ee9b346caa2fdc1c09026a2144a".into(),
                                published_at: "2023-12-09T15:56:29.180282477+00:00".into(),
                                published_by: "bob".into(),
                                requires: "".into(),
                                authors: vec!["alice".into()]
                            }
                        ]
                    },
                    PackageData {
                        name: "b_package".into(),
                        description: "".into(),
                        releases: vec![]
                    },
                    PackageData {
                        name: "c_package".into(),
                        description: "".into(),
                        releases: vec![
                            ReleaseData {
                                version: "0.1.0".into(),
                                filename: "c_package-0.1.0".into(),
                                url: "https://example.com/c_package-0.1.0".into(),
                                size: 123456,
                                checksum: "a8f515e9e2de99919d1a987733296aaa951a4ba2aa0f7014c510bdbd60dc0efd".into(),
                                published_at: "2023-12-15T15:56:29.180282477+00:00".into(),
                                published_by: "chuck".into(),
                                requires: "".into(),
                                authors: vec![]
                            }
                        ]
                    }
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners", "packages", "authors"))]
    async fn get_project_revision_ok_current(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_project_revision(42, 3).await.unwrap(),
            ProjectData {
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 3,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-12-14T15:50:06.419538067+00:00".into(),
                tags: vec![],
                game: GameData {
                    title: "A Game of Tests".into(),
                    title_sort_key: "Game of Tests, A".into(),
                    publisher: "Test Game Company".into(),
                    year: "1979".into()
                },
                readme: "".into(),
                image: None,
                owners: vec!["alice".into(), "bob".into()],
                packages: vec![
                    PackageData {
                        name: "a_package".into(),
                        description: "".into(),
                        releases: vec![
                            ReleaseData {
                                version: "1.2.4".into(),
                                filename: "a_package-1.2.4".into(),
                                url: "https://example.com/a_package-1.2.4".into(),
                                size: 5678,
                                checksum: "79fdd8fe3128f818e446e919cce5dcfb81815f8f4341c53f4d6b58ded48cebf2".into(),
                                published_at: "2023-12-10T15:56:29.180282477+00:00".into(),
                                published_by: "alice".into(),
                                requires: "".into(),
                                authors: vec!["alice".into(), "bob".into()]
                            },
                            ReleaseData {
                                version: "1.2.3".into(),
                                filename: "a_package-1.2.3".into(),
                                url: "https://example.com/a_package-1.2.3".into(),
                                size: 1234,
                                checksum: "c0e0fa7373a12b45a91e4f4d4e2e186442fc6ee9b346caa2fdc1c09026a2144a".into(),
                                published_at: "2023-12-09T15:56:29.180282477+00:00".into(),
                                published_by: "bob".into(),
                                requires: "".into(),
                                authors: vec!["alice".into()]
                            }
                        ]
                    },
                    PackageData {
                        name: "b_package".into(),
                        description: "".into(),
                        releases: vec![]
                    },
                    PackageData {
                        name: "c_package".into(),
                        description: "".into(),
                        releases: vec![]
                    }
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners", "packages"))]
    async fn get_project_revision_ok_old(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_project_revision(42, 1).await.unwrap(),
            ProjectData {
                name: "test_game".into(),
                description: "Brian's Trademarked Game of Being a Test Case".into(),
                revision: 1,
                created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                modified_at: "2023-11-12T15:50:06.419538067+00:00".into(),
                tags: vec![],
                game: GameData {
                    title: "A Game of Tests".into(),
                    title_sort_key: "Game of Tests, A".into(),
                    publisher: "Test Game Company".into(),
                    year: "1978".into()
                },
                readme: "".into(),
                image: None,
                owners: vec!["alice".into(), "bob".into()],
                packages: vec![
                    PackageData {
                        name: "b_package".into(),
                        description: "".into(),
                        releases: vec![]
                    },
                    PackageData {
                        name: "c_package".into(),
                        description: "".into(),
                        releases: vec![]
                    }
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn create_project_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let user = User("bob".into());
        let proj = Project("newproj".into());
        let data = ProjectData {
            name: proj.0.clone(),
            description: "A New Game".into(),
            revision: 1,
            created_at: NOW.into(),
            modified_at: NOW.into(),
            tags: vec![],
            game: GameData {
                title: "Some New Game".into(),
                title_sort_key: "Some New Game".into(),
                publisher: "XYZ Games".into(),
                year: "1999".into()
            },
            readme: "".into(),
            image: None,
            owners: vec!["bob".into()],
            packages: vec![]
        };

        let cdata = ProjectDataPost {
            description: data.description.clone(),
            tags: vec![],
            game: GameData {
                title: data.game.title.clone(),
                title_sort_key: data.game.title_sort_key.clone(),
                publisher: data.game.publisher.clone(),
                year: data.game.year.clone()
            },
            readme: "".into(),
            image: None
        };

        core.create_project(&user, &proj.0, &cdata).await.unwrap();
        let proj_id = core.get_project_id(&proj).await.unwrap();
        assert_eq!(core.get_project(proj_id.0).await.unwrap(), data);
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn update_project_ok(pool: Pool) {
        let core = make_core(pool, fake_now);

        let proj = Project("test_game".into());
        let new_data = ProjectData {
            name: proj.0.clone(),
            description: "new description".into(),
            revision: 4,
            created_at: "2023-11-12T15:50:06.419538067+00:00".into(),
            modified_at: NOW.into(),
            tags: vec![],
            game: GameData {
                title: "Some New Game".into(),
                title_sort_key: "Some New Game".into(),
                publisher: "XYZ Games".into(),
                year: "1999".into()
            },
            readme: "".into(),
            image: None,
            owners: vec!["bob".into()],
            packages: vec![]
        };

        let cdata = ProjectDataPatch {
            description: Some(new_data.description.clone()),
            tags: Some(vec![]),
            game: GameDataPatch {
                title: Some(new_data.game.title.clone()),
                title_sort_key: Some(new_data.game.title_sort_key.clone()),
                publisher: Some(new_data.game.publisher.clone()),
                year: Some(new_data.game.year.clone())
            },
            readme: Some("".into()),
            image: None
        };

        let proj_id = core.get_project_id(&proj).await.unwrap();
        let old_data = core.get_project(proj_id.0).await.unwrap();
        core.update_project(&Owner("bob".into()), 42, &cdata).await.unwrap();
        // project has new data
        assert_eq!(core.get_project(proj_id.0).await.unwrap(), new_data);
        // old data is kept as a revision
        assert_eq!(
            core.get_project_revision(proj_id.0, 3).await.unwrap(),
            old_data
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_release_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_release(42, 1).await.unwrap(),
            "https://example.com/a_package-1.2.4"
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_release_version_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        let version = "1.2.3".parse::<Version>().unwrap();
        assert_eq!(
            core.get_release_version(42, 1, &version).await.unwrap(),
            "https://example.com/a_package-1.2.3"
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_release_version_not_a_version(pool: Pool) {
        let core = make_core(pool, fake_now);
        let version = "1.0.0".parse::<Version>().unwrap();
        assert_eq!(
            core.get_release_version(42, 1, &version).await.unwrap_err(),
            AppError::NotAVersion
        );
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn get_owners_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_owners(42).await.unwrap(),
            Users { users: vec![User("bob".into())] }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn user_is_owner_true(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert!(core.user_is_owner(&User("bob".into()), 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn user_is_owner_false(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert!(!core.user_is_owner(&User("alice".into()), 42).await.unwrap());
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn add_owners_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        let users = Users { users: vec![User("alice".into())] };
        core.add_owners(&users, 42).await.unwrap();
        assert_eq!(
            core.get_owners(42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "two_owners"))]
    async fn remove_owners_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        let users = Users { users: vec![User("bob".into())] };
        core.remove_owners(&users, 42).await.unwrap();
        assert_eq!(
            core.get_owners(42).await.unwrap(),
            Users { users: vec![User("alice".into())] }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "one_owner"))]
    async fn remove_owners_fail_if_last(pool: Pool) {
        let core = make_core(pool, fake_now);
        let users = Users { users: vec![User("bob".into())] };
        assert_eq!(
            core.remove_owners(&users, 1).await.unwrap_err(),
            AppError::CannotRemoveLastOwner
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn get_players_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_players(42).await.unwrap(),
            Users {
                users: vec![
                    User("alice".into()),
                    User("bob".into())
                ]
            }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "players"))]
    async fn add_player_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        core.add_player(&User("chuck".into()), 42).await.unwrap();
        assert_eq!(
            core.get_players(42).await.unwrap(),
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
    async fn remove_player_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        core.remove_player(&User("bob".into()), 42).await.unwrap();
        assert_eq!(
            core.get_players(42).await.unwrap(),
            Users { users: vec![User("alice".into())] }
        );
    }

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_ok(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_image(42, "img.png").await.unwrap(),
            "https://example.com/images/img.png"
        );
    }

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_not_a_project(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_image(1, "img.png").await.unwrap_err(),
            AppError::NotFound
        );
    }

    #[sqlx::test(fixtures("users", "projects", "images"))]
    async fn get_image_not_an_image(pool: Pool) {
        let core = make_core(pool, fake_now);
        assert_eq!(
            core.get_image(42, "bogus").await.unwrap_err(),
            AppError::NotFound
        );
    }
}
