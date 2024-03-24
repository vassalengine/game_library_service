use sqlx::{
    Acquire, Executor,
    sqlite::Sqlite
};

use crate::{
    core::CoreError,
    db::PackageRow,
    model::{Owner, PackageDataPost, Project},
    sqlite::projects::update_project_non_project_data
};

pub async fn get_packages<'e, E>(
    ex: E,
    proj: Project
) -> Result<Vec<PackageRow>, CoreError>
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
            proj.0
        )
       .fetch_all(ex)
       .await?
    )
}

pub async fn get_packages_at<'e, E>(
    ex: E,
    proj: Project,
    date: i64
) -> Result<Vec<PackageRow>, CoreError>
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
            proj.0,
            date
        )
       .fetch_all(ex)
       .await?
    )
}

pub async fn create_package<'a, A>(
    conn: A,
    owner: Owner,
    proj: Project,
    pkg: &str,
    pkg_data: &PackageDataPost,
    now: i64
) -> Result<(), CoreError>
where
    A: Acquire<'a, Database = Sqlite>
{
    let mut tx = conn.begin().await?;

    sqlx::query!(
        "
INSERT INTO packages (
    project_id,
    name,
    created_at,
    created_by
)
VALUES (?, ?, ?, ?)
            ",
            proj.0,
            pkg,
            now,
            owner.0
    )
    .execute(&mut *tx)
    .await?;

    // update project to reflect the change
    update_project_non_project_data(&mut tx, owner, proj, now).await?;

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    type Pool = sqlx::Pool<Sqlite>;

        #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_ok(pool: Pool) {
        assert_eq!(
            get_packages(&pool, Project(42)).await.unwrap(),
            vec![
                PackageRow {
                    package_id: 1,
                    name: "a_package".into(),
                    created_at: 1702137389180282477
                },
                PackageRow {
                    package_id: 2,
                    name: "b_package".into(),
                    created_at: 1667750189180282477
                },
                PackageRow {
                    package_id: 3,
                    name: "c_package".into(),
                    created_at: 1699286189180282477
                }
            ]
        );
    }

// TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_not_a_project(pool: Pool) {
        assert_eq!(
            get_packages(&pool, Project(0)).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_none(pool: Pool) {
        let date = 0;
        assert_eq!(
            get_packages_at(&pool, Project(42), date).await.unwrap(),
            vec![]
        );
    }

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_some(pool: Pool) {
        let date = 1672531200000000000;
        assert_eq!(
            get_packages_at(&pool, Project(42), date).await.unwrap(),
            vec![
                PackageRow {
                    package_id: 2,
                    name: "b_package".into(),
                    created_at: 1667750189180282477
                }
            ]
        );
    }

    // TODO: can we tell when the project doesn't exist?
    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn get_packages_at_not_a_project(pool: Pool) {
        let date = 16409952000000000;
        assert_eq!(
            get_packages_at(&pool, Project(0), date).await.unwrap(),
            vec![]
        );
    }

// TODO: test create_package not a project
// TODO: test create_package duplicate name

    #[sqlx::test(fixtures("users", "projects", "packages"))]
    async fn create_package_ok(pool: Pool) {
        assert_eq!(
            get_packages(&pool, Project(6)).await.unwrap(),
            []
        );

        create_package(
            &pool,
            Owner(1),
            Project(6),
            "newpkg",
            &PackageDataPost {
                description: "".into()
            },
            1699804206419538067
        ).await.unwrap();

// TODO: also check that a revision is made?

        assert_eq!(
            get_packages(&pool, Project(6)).await.unwrap(),
            [
                PackageRow {
                    package_id: 4,
                    name: "newpkg".into(),
                    created_at: 1699804206419538067
                }
            ]
        );
    }
}
