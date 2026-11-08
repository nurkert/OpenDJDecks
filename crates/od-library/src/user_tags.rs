//! Tags: user-defined tag categories (such as "Mood" or "Situation") with
//! free values (such as "Peak" or "Warm-up") attached to tracks.

use crate::journal::EntityState;
use crate::{Library, Result};
use od_core::TrackId;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UserTag {
    pub category: String,
    pub value: String,
}

impl UserTag {
    pub fn new(category: &str, value: &str) -> Self {
        Self {
            category: category.trim().to_string(),
            value: value.trim().to_string(),
        }
    }
}

pub(crate) fn read(conn: &Connection, track: TrackId) -> Result<Vec<UserTag>> {
    let mut stmt = conn.prepare_cached(
        "SELECT category, value FROM my_tag WHERE track_id = ?1 ORDER BY category, value",
    )?;
    let rows = stmt.query_map([track.0], |r| {
        Ok(UserTag {
            category: r.get(0)?,
            value: r.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub(crate) fn write(conn: &Connection, track: TrackId, tags: &[UserTag]) -> Result<()> {
    conn.execute("DELETE FROM my_tag WHERE track_id = ?1", [track.0])?;
    let mut stmt = conn.prepare_cached(
        "INSERT OR IGNORE INTO my_tag (track_id, category, value) VALUES (?1, ?2, ?3)",
    )?;
    for t in tags
        .iter()
        .filter(|t| !t.category.is_empty() && !t.value.is_empty())
    {
        stmt.execute(params![track.0, t.category, t.value])?;
    }
    Ok(())
}

impl Library {
    /// The tag values of one track.
    pub fn user_tags(&self, track: TrackId) -> Result<Vec<UserTag>> {
        read(&self.conn, track)
    }

    /// Tag values of every tagged track.
    pub fn all_user_tags(&self) -> Result<HashMap<TrackId, Vec<UserTag>>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT track_id, category, value FROM my_tag ORDER BY category, value",
        )?;
        let mut out: HashMap<TrackId, Vec<UserTag>> = HashMap::new();
        for row in stmt.query_map([], |r| {
            Ok((
                TrackId(r.get(0)?),
                UserTag {
                    category: r.get(1)?,
                    value: r.get(2)?,
                },
            ))
        })? {
            let (t, tag) = row?;
            out.entry(t).or_default().push(tag);
        }
        Ok(out)
    }

    /// Every category with its values and how many tracks carry each value.
    pub fn user_tag_catalog(&self) -> Result<BTreeMap<String, BTreeMap<String, u32>>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT category, value, count(*) FROM my_tag GROUP BY category, value",
        )?;
        let mut out: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        for row in stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })? {
            let (c, v, n) = row?;
            out.entry(c).or_default().insert(v, n);
        }
        Ok(out)
    }

    /// User edit: replace the tag values of a track.
    pub fn set_user_tags(&mut self, track: TrackId, tags: Vec<UserTag>) -> Result<()> {
        self.track(track)?;
        self.edit("Edit Tags", vec![EntityState::UserTags { track, tags }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::track;

    #[test]
    fn user_tags_are_undoable() {
        let mut lib = Library::open_in_memory().unwrap();
        let a = track(&mut lib, "A", None, None);
        lib.set_user_tags(
            a,
            vec![UserTag::new("Mood", "Dark"), UserTag::new("Situation", "Peak")],
        )
        .unwrap();
        assert_eq!(lib.user_tags(a).unwrap().len(), 2);
        assert_eq!(lib.user_tag_catalog().unwrap()["Mood"]["Dark"], 1);
        lib.set_user_tags(a, vec![UserTag::new("Mood", "Warm")])
            .unwrap();
        assert_eq!(
            lib.all_user_tags().unwrap()[&a],
            vec![UserTag::new("Mood", "Warm")]
        );
        lib.undo().unwrap();
        assert_eq!(lib.user_tags(a).unwrap().len(), 2);
    }
}
