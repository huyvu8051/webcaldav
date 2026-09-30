use crate::storage::ConnectionConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarSummary {
    pub url: String,
    pub display_name: String,
    pub ctag: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EventsFetch {
    pub events: Vec<EventSummary>,
    pub latency_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventSummary {
    pub uid: String,
    pub url: String,
    pub etag: Option<String>,
    pub calendar_url: String,
    pub summary: String,
    pub description: String,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    pub location: String,
    pub priority: Option<u8>,
    pub busy: Option<bool>,
    pub reminder_minutes: Option<i64>,
    pub travel_minutes: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct NewEvent {
    pub summary: String,
    pub description: String,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    pub location: String,
    pub priority: Option<u8>,
    pub busy: Option<bool>,
    pub reminder_minutes: Option<i64>,
    pub travel_minutes: Option<i64>,
}

#[cfg(target_arch = "wasm32")]
mod live {
    use super::*;
    use crate::ics::{build_vevent, from_ical_datetime, parse_vevent, to_ical_date, to_ical_datetime, VEventFields};
    use rsdav::{create_dav_client, AuthMethod, DavAccountType, DavCalendar, DavCalendarObject, DavClient, DavCredentials};

    fn credentials(config: &ConnectionConfig) -> DavCredentials {
        DavCredentials {
            username: Some(config.username.clone()),
            password: Some(config.password.clone()),
            ..Default::default()
        }
    }

    async fn login(config: &ConnectionConfig) -> Result<DavClient, String> {
        create_dav_client(config.server_url.clone(), credentials(config), AuthMethod::Basic, DavAccountType::Caldav)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_calendars(config: &ConnectionConfig) -> Result<Vec<CalendarSummary>, String> {
        let client = login(config).await?;
        let calendars = client.fetch_calendars(None).await.map_err(|e| e.to_string())?;
        Ok(calendars
            .into_iter()
            .map(|c| CalendarSummary {
                url: c.url,
                display_name: c.display_name.unwrap_or_else(|| "(untitled calendar)".to_string()),
                ctag: c.ctag,
                color: c.calendar_color,
            })
            .collect())
    }

    fn parse_event(calendar_url: &str, data: &str, obj_url: String, etag: Option<String>) -> Option<EventSummary> {
        let parsed = parse_vevent(data)?;
        Some(EventSummary {
            uid: parsed.uid,
            url: obj_url,
            etag,
            calendar_url: calendar_url.to_string(),
            summary: parsed.summary,
            description: parsed.description,
            start: from_ical_datetime(&parsed.start),
            end: from_ical_datetime(&parsed.end),
            all_day: parsed.all_day,
            location: parsed.location,
            priority: parsed.priority,
            busy: parsed.busy,
            reminder_minutes: parsed.reminder_minutes,
            travel_minutes: parsed.travel_minutes,
        })
    }

    pub async fn list_events_for_calendars(config: &ConnectionConfig, calendar_urls: &[String]) -> Result<EventsFetch, String> {
        let started = chrono::Utc::now();
        let client = login(config).await?;
        let mut events = Vec::new();
        for calendar_url in calendar_urls {
            let calendar = DavCalendar { url: calendar_url.clone(), ..Default::default() };
            let objects = client.fetch_calendar_objects(&calendar, None, None).await.map_err(|e| e.to_string())?;
            events.extend(objects.into_iter().filter_map(|obj| {
                let data = obj.data?;
                parse_event(calendar_url, &data, obj.url, obj.etag)
            }));
        }
        let latency_ms = (chrono::Utc::now() - started).num_milliseconds();
        Ok(EventsFetch { events, latency_ms })
    }

    fn new_uid() -> String {
        let random = (js_sys::Math::random() * 1_000_000_000.0) as u64;
        let now = js_sys::Date::now() as u64;
        format!("webcaldav-{now}-{random}@opendiy.vn")
    }

    fn now_dtstamp() -> String {
        let date = js_sys::Date::new_0();
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            date.get_utc_full_year(),
            date.get_utc_month() + 1,
            date.get_utc_date(),
            date.get_utc_hours(),
            date.get_utc_minutes(),
            date.get_utc_seconds(),
        )
    }

    fn ical_bounds(event: &NewEvent) -> (String, String) {
        if event.all_day {
            let start = to_ical_date(&event.start);
            let end_date = chrono::NaiveDate::parse_from_str(&event.end, "%Y-%m-%d")
                .ok()
                .map(|d| d + chrono::Duration::days(1))
                .map(|d| d.format("%Y%m%d").to_string())
                .unwrap_or_else(|| start.clone());
            (start, end_date)
        } else {
            (to_ical_datetime(&event.start), to_ical_datetime(&event.end))
        }
    }

    fn build_ical(uid: &str, new_event: &NewEvent) -> String {
        let (start, end) = ical_bounds(new_event);
        build_vevent(&VEventFields {
            uid,
            dtstamp: &now_dtstamp(),
            start: &start,
            end: &end,
            all_day: new_event.all_day,
            summary: &new_event.summary,
            description: &new_event.description,
            location: &new_event.location,
            priority: new_event.priority,
            busy: new_event.busy,
            reminder_minutes: new_event.reminder_minutes,
            travel_minutes: new_event.travel_minutes,
        })
    }

    pub async fn create_event(config: &ConnectionConfig, calendar_url: &str, new_event: NewEvent) -> Result<(), String> {
        let client = login(config).await?;
        let calendar = DavCalendar { url: calendar_url.to_string(), ..Default::default() };
        let uid = new_uid();
        let ical = build_ical(&uid, &new_event);
        client
            .create_calendar_object(&calendar, &ical, &format!("{uid}.ics"))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Updates an event in place, unless `target_calendar_url` differs from the
    /// event's current calendar — CalDAV has no portable "move" operation, so a
    /// cross-calendar move is done as delete-then-create in the target collection.
    pub async fn update_event(config: &ConnectionConfig, event: &EventSummary, target_calendar_url: &str, updated: NewEvent) -> Result<(), String> {
        let client = login(config).await?;
        if target_calendar_url == event.calendar_url {
            let ical = build_ical(&event.uid, &updated);
            let object = DavCalendarObject { url: event.url.clone(), etag: event.etag.clone(), data: Some(ical) };
            client.update_calendar_object(&object).await.map_err(|e| e.to_string())?;
        } else {
            let old = DavCalendarObject { url: event.url.clone(), etag: event.etag.clone(), data: None };
            client.delete_calendar_object(&old).await.map_err(|e| e.to_string())?;
            let calendar = DavCalendar { url: target_calendar_url.to_string(), ..Default::default() };
            let ical = build_ical(&event.uid, &updated);
            client
                .create_calendar_object(&calendar, &ical, &format!("{}.ics", event.uid))
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn delete_event(config: &ConnectionConfig, event: &EventSummary) -> Result<(), String> {
        let client = login(config).await?;
        let object = DavCalendarObject { url: event.url.clone(), etag: event.etag.clone(), data: None };
        client.delete_calendar_object(&object).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
pub use live::*;

#[cfg(not(target_arch = "wasm32"))]
mod stub {
    use super::*;

    const NOT_AVAILABLE: &str = "CalDAV operations only run in the browser (wasm32 target)";

    pub async fn list_calendars(_config: &ConnectionConfig) -> Result<Vec<CalendarSummary>, String> {
        Err(NOT_AVAILABLE.to_string())
    }

    pub async fn list_events_for_calendars(_config: &ConnectionConfig, _calendar_urls: &[String]) -> Result<EventsFetch, String> {
        Err(NOT_AVAILABLE.to_string())
    }

    pub async fn create_event(_config: &ConnectionConfig, _calendar_url: &str, _new_event: NewEvent) -> Result<(), String> {
        Err(NOT_AVAILABLE.to_string())
    }

    pub async fn update_event(_config: &ConnectionConfig, _event: &EventSummary, _target_calendar_url: &str, _updated: NewEvent) -> Result<(), String> {
        Err(NOT_AVAILABLE.to_string())
    }

    pub async fn delete_event(_config: &ConnectionConfig, _event: &EventSummary) -> Result<(), String> {
        Err(NOT_AVAILABLE.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use stub::*;
