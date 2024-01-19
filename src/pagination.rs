use base64::{Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    str,
    num::NonZeroU8
};

use crate::errors::AppError;

// TODO: private fields various places

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "&str")]
#[repr(transparent)]
pub struct Limit(NonZeroU8);

impl Limit {
    pub const fn new(limit: u8) -> Option<Limit> {
        match limit {
            limit if limit > 100 => None,
            limit => match NonZeroU8::new(limit) {
                Some(n) => Some(Limit(n)),
                None => None
            }
        }
    }

    pub const fn get(self) -> u8 {
        self.0.get()
    }
}

impl Default for Limit {
    fn default() -> Self {
        Limit::new(10).expect("0 < 10 <= 100")
    }
}

impl TryFrom<&str> for Limit {
    type Error = AppError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.parse::<u8>() {
            Ok(n) => Limit::new(n).ok_or(AppError::LimitOutOfRange),
            Err(_) => Err(AppError::MalformedQuery)
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "&str")]
pub enum Anchor {
    #[default]
    Start,
    Before(u32, String),
    After(u32, String),
    End
}

impl From<Anchor> for String {
    fn from(value: Anchor) -> Self {
        match value {
            Anchor::Start => "s".to_string(),
            Anchor::Before(i, n) => format!("b:{}:{}", i, n),
            Anchor::After(i, n) => format!("a:{}:{}", i, n),
            Anchor::End => "e".to_string()
        }
    }
}

impl TryFrom<&str> for Anchor {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "s" => Ok(Anchor::Start),
            "e" => Ok(Anchor::End),
            s => {
                let v: Vec<&str> = s.splitn(3, ':').collect();
                if v.len() == 3 {
                    let i = v[1].parse::<u32>()
                        .or(Err(AppError::MalformedQuery))?;
                    match v[0] {
                        "b" => Ok(Anchor::Before(i, v[2].into())),
                        "a" => Ok(Anchor::After(i, v[2].into())),
                        _ => Err(AppError::MalformedQuery)
                    }
                }
                else {
                    Err(AppError::MalformedQuery)
                }
            }
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "&str")]
pub enum OrderBy {
    #[default]
    ProjectName,
    GameTitle
}

impl From<OrderBy> for String {
    fn from(value: OrderBy) -> Self {
        match value {
            OrderBy::ProjectName => "p".to_string(),
            OrderBy::GameTitle => "t".to_string()
        }
    }
}

impl TryFrom<&str> for OrderBy {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "p" => Ok(OrderBy::ProjectName),
            "t" => Ok(OrderBy::GameTitle),
            _ => Err(AppError::MalformedQuery)
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "&str")]
pub struct Seek {
    pub anchor: Anchor,
    pub order_by: OrderBy
}

impl From<Seek> for String {
    fn from(value: Seek) -> Self {
        let s = String::from(value.order_by) + &String::from(value.anchor);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s)
    }
}

impl TryFrom<&str> for Seek {
    type Error = AppError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let buf = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|_| AppError::MalformedQuery)?;

        let d = str::from_utf8(&buf)
            .map_err(|_| AppError::MalformedQuery)?;

        let mut i = d.chars();
        let c = i.next().ok_or(AppError::MalformedQuery)?.to_string();
        let order_by = OrderBy::try_from(c.as_str())?;
        let anchor = Anchor::try_from(i.as_str())?;

        Ok(
            Seek {
                anchor,
                order_by
            }
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub seek: Option<Seek>,
    pub limit: Option<Limit>
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct SeekLink(String);

impl SeekLink {
    pub fn new(seek: Seek) -> SeekLink {
        SeekLink("?seek=".to_string() + &String::from(seek))
    }
}

impl From<Seek> for SeekLink {
    fn from(seek: Seek) -> Self {
        SeekLink::new(seek)
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Pagination {
    pub prev_page: Option<SeekLink>,
    pub next_page: Option<SeekLink>,
    pub total: i32
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn size_of_limit() {
        assert_eq!(
            std::mem::size_of::<Limit>(),
            std::mem::size_of::<u8>()
        );
    }

    #[test]
    fn string_to_limit_zero_err() {
        assert!(Limit::try_from("0").is_err());
    }

    #[test]
    fn string_to_limit_one_ok() {
        assert_eq!(
            Limit::try_from("1").unwrap(),
            Limit::new(1).unwrap()
        );
    }

    #[test]
    fn string_to_limit_one_hundred_ok() {
        assert_eq!(
            Limit::try_from("100").unwrap(),
            Limit::new(100).unwrap()
        );
    }

    #[test]
    fn string_to_limit_one_hundred_one_err() {
        assert!(Limit::try_from("101").is_err());
    }

    #[test]
    fn seek_to_string_start() {
        assert_eq!(
            &String::from(
                Seek {
                    anchor: Anchor::Start,
                    order_by: OrderBy::ProjectName
                }
            ),
            "cHM"
        );
    }

/*
    #[test]
    fn xxx() {
        assert_eq!(
            &String::from(
                Seek {
                    anchor: Anchor::Start,
                    order_by: OrderBy::GameTitle
                }
            ),
            "cHM6"
        );
    }
*/

    #[test]
    fn seek_to_string_end() {
        assert_eq!(
            &String::from(
                Seek {
                    anchor: Anchor::End,
                    order_by: OrderBy::ProjectName
                }
            ),
            "cGU"
        );
    }

    #[test]
    fn seek_to_string_before() {
        assert_eq!(
            &String::from(
                Seek {
                    anchor: Anchor::Before(0, "abc".into()),
                    order_by: OrderBy::ProjectName
                }
            ),
            "cGI6MDphYmM"
        );
    }

    #[test]
    fn seek_to_string_after() {
        assert_eq!(
            &String::from(
                Seek {
                    anchor: Anchor::After(0, "abc".into()),
                    order_by: OrderBy::ProjectName
                }
            ),
            "cGE6MDphYmM"
        );
    }

    #[test]
    fn string_to_seek_start() {
        assert_eq!(
            Seek::try_from("cHM").unwrap(),
            Seek {
                anchor: Anchor::Start,
                order_by: OrderBy::ProjectName
            }
        );
    }

    #[test]
    fn string_to_seek_end() {
        assert_eq!(
            Seek::try_from("cGU").unwrap(),
            Seek {
                anchor: Anchor::End,
                order_by: OrderBy::ProjectName
            }
        );
    }

    #[test]
    fn string_to_seek_before() {
        assert_eq!(
            Seek::try_from("cGI6MDphYmM").unwrap(),
            Seek {
                anchor: Anchor::Before(0, "abc".into()),
                order_by: OrderBy::ProjectName
            }
        );
    }

    #[test]
    fn string_to_seek_after() {
        assert_eq!(
            Seek::try_from("cGE6MDphYmM").unwrap(),
            Seek {
                anchor: Anchor::After(0, "abc".into()),
                order_by: OrderBy::ProjectName
            }
        );
    }

    #[test]
    fn string_to_seek_err() {
        assert!(Seek::try_from("$$$").is_err());
    }
}
