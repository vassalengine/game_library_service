use serde::{Deserialize, Serialize};

use crate::pagination::Pagination;

// TODO: rationalize struct naming---names should reflect whether the
// structs are input or ouptut

// TODO: Maybe User should be a newtype so that you can't construct one
// without having verified that the user exists

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct User(pub i64);

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Users {
    pub users: Vec<String>
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Package(pub i64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Project(pub i64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Owner(pub i64);

#[derive(Debug, Eq, PartialEq)]
pub struct Owned(pub Owner, pub Project);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameData {
    pub title: String,
    pub title_sort_key: String,
    pub publisher: String,
    pub year: String
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseData {
    pub version: String,
    pub filename: String,
    pub url: String,
    pub size: i64,
    pub checksum: String,
    pub published_at: String,
    pub published_by: String,
    pub requires: String,
    pub authors: Vec<String>
}

// TODO: probably needs slug
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageData {
    pub name: String,
    pub description: String,
    pub releases: Vec<ReleaseData>
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageDataPost {
// TODO: display name?
//    pub name: String,
    pub description: String
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectData {
    pub name: String,
    pub description: String,
    pub revision: i64,
    pub created_at: String,
    pub modified_at: String,
    pub tags: Vec<String>,
    pub game: GameData,
    pub readme: String,
    pub image: Option<String>,
    pub owners: Vec<String>,
    pub packages: Vec<PackageData>
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameDataPatch {
    pub title: Option<String>,
    pub title_sort_key: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<String>
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MaybeProjectDataPatch {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub game: GameDataPatch,
    pub readme: Option<String>,
    pub image: Option<Option<String>>
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "MaybeProjectDataPatch")]
pub struct ProjectDataPatch {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub game: GameDataPatch,
    pub readme: Option<String>,
    pub image: Option<Option<String>>
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error("invalid data {0:?}")]
pub struct ProjectDataPatchError(MaybeProjectDataPatch);

impl TryFrom<MaybeProjectDataPatch> for ProjectDataPatch {
    type Error = ProjectDataPatchError;

    fn try_from(m: MaybeProjectDataPatch) -> Result<Self, Self::Error> {
        // at least one element must be present to be a valid request
        if m.description.is_none() &&
           m.tags.is_none() &&
           m.game.title.is_none() &&
           m.game.title_sort_key.is_none() &&
           m.game.publisher.is_none() &&
           m.game.year.is_none() &&
           m.readme.is_none() &&
           m.image.is_none()
        {
            Err(ProjectDataPatchError(m))
        }
        else {
            Ok(
                ProjectDataPatch {
                    description: m.description,
                    tags: m.tags,
                    game: m.game,
                    readme: m.readme,
                    image: m.image
                }
            )
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectDataPost {
    pub description: String,
    pub tags: Vec<String>,
    pub game: GameData,
    pub readme: String,
    pub image: Option<String>
}

// TODO: maybe use a date type for ctime, mtime?

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectSummary {
    pub name: String,
    pub description: String,
    pub revision: i64,
    pub created_at: String,
    pub modified_at: String,
    pub tags: Vec<String>,
    pub game: GameData
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Projects {
    pub projects: Vec<ProjectSummary>,
    pub meta: Pagination
}
