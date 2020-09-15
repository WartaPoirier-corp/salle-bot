use crate::calendar::Rooms;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct CalendarSync {
    url: String,        // TODO implement periodic sync
    next_sync: Instant, // TODO too
    rooms: Arc<Mutex<Arc<Rooms>>>,
}

impl CalendarSync {
    /// First fetch panics if it fails as the "rooms" field cannot be empty
    pub async fn new(url: String) -> Self {
        let rooms = fetch::fetch(&url)
            .await
            .expect("rooms initialization failed");

        Self {
            url,
            next_sync: Instant::now() + Duration::from_secs(24 * 3600),
            rooms: Arc::new(Mutex::new(rooms.into())),
        }
    }

    pub fn get(&self) -> Arc<Rooms> {
        Arc::clone(&self.rooms.lock().unwrap())
    }
}

mod fetch {
    use crate::calendar::*;
    use chrono::{DateTime, NaiveDateTime, Utc};
    use ical::parser::ical::component::IcalEvent;
    use std::collections::hash_map::{Entry, HashMap};
    use std::convert::TryFrom;
    use std::io::BufReader;
    use std::ops::RangeInclusive;

    const DATE_FORMAT: &str = "%Y%m%dT%H%M%SZ";

    #[derive(Clone, Debug)]
    pub struct CalEntry {
        span: RangeInclusive<DateTime<Utc>>,
        room: Room,
    }

    impl TryFrom<&IcalEvent> for CalEntry {
        type Error = ();

        fn try_from(src: &IcalEvent) -> Result<Self, ()> {
            let start = DateTime::from_utc(
                NaiveDateTime::parse_from_str(find_prop(src, "DTSTART")?, DATE_FORMAT)
                    .map_err(|e| println!("{:?}", e))?,
                Utc,
            );

            let end = DateTime::from_utc(
                NaiveDateTime::parse_from_str(find_prop(src, "DTEND")?, DATE_FORMAT)
                    .map_err(|e| println!("{:?}", e))?,
                Utc,
            );

            let room = find_prop(src, "LOCATION")?;

            if !room.starts_with("DLST-") {
                return Err(());
            }

            Ok(CalEntry {
                span: start..=end,
                room: Room::parse(room.to_owned()),
            })
        }
    }

    fn find_prop<'a>(event: &'a IcalEvent, property: &'static str) -> Result<&'a str, ()> {
        let prop = event.properties.iter().find(|prop| property == prop.name);
        prop.and_then(|p| p.value.as_deref()).ok_or(())
    }

    #[derive(Debug)]
    pub enum Error {
        Reqwest(reqwest::Error),
        ParseNone,
        Parse(ical::parser::ParserError),
    }

    impl From<reqwest::Error> for Error {
        fn from(src: reqwest::Error) -> Self {
            Self::Reqwest(src)
        }
    }

    pub async fn fetch(url: impl AsRef<str>) -> Result<Box<Rooms>, Error> {
        let feed = reqwest::get(url.as_ref()).await?.bytes().await?;

        let feed = BufReader::new(feed.as_ref());

        let mut parser = ical::IcalParser::new(feed);

        let calendar = parser
            .next()
            .ok_or(Error::ParseNone)?
            .map_err(|err| Error::Parse(err))?;

        let cal_entries: Vec<_> = calendar
            .events
            .into_iter() // TODO use rayon
            .filter_map(|event| CalEntry::try_from(&event).ok())
            .collect();

        let mut rooms = HashMap::new();

        for cal_entry in cal_entries {
            let mut entry = rooms.entry(cal_entry.room);

            let cell = match entry {
                Entry::Occupied(ref mut entry) => entry.get_mut(),
                Entry::Vacant(entry) => entry.insert(Vec::new()),
            };

            cell.push(cal_entry.span);
        }

        Ok(Box::new(Rooms { rooms }))
    }
}
