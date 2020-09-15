use arraystring::{typenum::U4, ArrayString};
use chrono::{DateTime, Utc};
use regex::Regex;
use serenity::static_assertions::_core::ops::RangeInclusive;
use std::collections::hash_map::HashMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Room {
    name: ArrayString<U4>,
}

impl Room {
    pub fn parse(name: impl AsRef<str>) -> Self {
        lazy_static! {
            static ref RE_ROOM: Regex = Regex::new(r"([A-F]\d{3})|(F\d)").unwrap();
        }

        let name = &RE_ROOM
            .captures_iter(name.as_ref())
            .next()
            .expect(&format!("error executing regex with {}", name.as_ref()))[0];

        Self { name: name.into() }
    }

    pub fn bat(&self) -> char {
        self.name.chars().next().unwrap()
    }
}

impl std::fmt::Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug)]
pub struct Rooms {
    pub rooms: HashMap<Room, Vec<RangeInclusive<DateTime<Utc>>>>,
}

impl Rooms {
    pub fn rooms(&self) -> Vec<&Room> {
        self.rooms.keys().collect()
    }

    pub fn rooms_and_timetable(&self) -> Vec<(&Room, &Vec<RangeInclusive<DateTime<Utc>>>)> {
        self.rooms.iter().collect()
    }
}
