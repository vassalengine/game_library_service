use serde::Deserialize;

use crate::{
    errors::AppError,
    pagination::{Anchor, Limit, Direction, SortBy, Seek}
};

#[derive(Debug, Default, Deserialize)]
pub struct MaybeProjectsParams {
    pub q: Option<String>,
    pub sort: Option<SortBy>,
    pub order: Option<Direction>,
    pub seek: Option<Seek>,
    pub limit: Option<Limit>
}

#[derive(Debug, Deserialize)]
pub enum SortOrSeek {
    Sort(SortBy, Direction),
    Seek(Seek)
}

impl Default for SortOrSeek {
    fn default() -> Self {
        SortOrSeek::Sort(SortBy::ProjectName, Direction::Ascending)
    }
}

impl From<SortOrSeek> for Seek {
    fn from(value: SortOrSeek) -> Self {
        match value {
            SortOrSeek::Sort(sort_by, dir) => Seek {
                sort_by,
                dir,
                anchor: Anchor::Start
            },
            SortOrSeek::Seek(seek) => seek
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(try_from = "MaybeProjectsParams")]
pub struct ProjectsParams {
    pub q: Option<String>,
    pub seek: Seek,
    pub limit: Limit
}

impl TryFrom<MaybeProjectsParams> for ProjectsParams {
    type Error = AppError;

    fn try_from(m: MaybeProjectsParams) -> Result<Self, Self::Error> {
        if (m.sort.is_some() || m.order.is_some()) && m.seek.is_some() {
            // sort, order are incompatible with seek
            Err(AppError::MalformedQuery)
        }
        else {
            let from = if let Some(seek) = m.seek {
                SortOrSeek::Seek(seek)
            }
            else {
                let sort = m.sort.unwrap_or(SortBy::ProjectName);
                let dir = m.order.unwrap_or_else(|| sort.default_direction());
                SortOrSeek::Sort(sort, dir)
            };

            Ok(
                ProjectsParams {
                    q: m.q,
                    limit: m.limit.unwrap_or_default(),
                    seek: Seek::from(from)
                }
            )
        }
    }
}
