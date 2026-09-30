use chrono::{Duration, NaiveDate};

pub struct ParsedEvent {
    pub uid: String,
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

pub struct VEventFields<'a> {
    pub uid: &'a str,
    pub dtstamp: &'a str,
    pub start: &'a str,
    pub end: &'a str,
    pub all_day: bool,
    pub summary: &'a str,
    pub description: &'a str,
    pub location: &'a str,
    pub priority: Option<u8>,
    pub busy: Option<bool>,
    pub reminder_minutes: Option<i64>,
    pub travel_minutes: Option<i64>,
}

pub fn to_ical_datetime(input: &str) -> String {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit() || *c == 'T').collect();
    format!("{digits}00")
}

pub fn to_ical_date(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn parse_yyyymmdd(v: &str) -> Option<NaiveDate> {
    if v.len() < 8 {
        return None;
    }
    NaiveDate::from_ymd_opt(v[0..4].parse().ok()?, v[4..6].parse().ok()?, v[6..8].parse().ok()?)
}

pub fn from_ical_datetime(value: &str) -> String {
    let v = value.trim_end_matches('Z');
    if v.len() >= 15 {
        format!(
            "{}-{}-{}T{}:{}",
            &v[0..4],
            &v[4..6],
            &v[6..8],
            &v[9..11],
            &v[11..13]
        )
    } else if v.len() == 8 {
        format!("{}-{}-{}", &v[0..4], &v[4..6], &v[6..8])
    } else {
        String::new()
    }
}

/// Duration string as used in VALARM TRIGGER values (e.g. `-PT15M`, `-P1D`,
/// `-PT1H30M`) to whole minutes. Ignores the sign (alarms are always in the
/// past relative to the event) and seconds/years/months, which this app
/// never emits and rarely sees in the wild for alarm triggers.
fn parse_duration_minutes(s: &str) -> Option<i64> {
    let s = s.trim_start_matches(['+', '-']);
    let s = s.strip_prefix('P')?;
    let (date_part, time_part) = match s.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let mut minutes = 0i64;
    let mut num = String::new();
    for c in date_part.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if c == 'W' {
            minutes += num.parse::<i64>().unwrap_or(0) * 7 * 24 * 60;
            num.clear();
        } else if c == 'D' {
            minutes += num.parse::<i64>().unwrap_or(0) * 24 * 60;
            num.clear();
        }
    }
    if let Some(time_part) = time_part {
        num.clear();
        for c in time_part.chars() {
            if c.is_ascii_digit() {
                num.push(c);
            } else if c == 'H' {
                minutes += num.parse::<i64>().unwrap_or(0) * 60;
                num.clear();
            } else if c == 'M' {
                minutes += num.parse::<i64>().unwrap_or(0);
                num.clear();
            } else if c == 'S' {
                num.clear();
            }
        }
    }
    Some(minutes)
}

fn format_trigger_minutes(minutes: i64) -> String {
    format!("-PT{minutes}M")
}

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

fn unescape_text(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\;", ";")
        .replace("\\,", ",")
        .replace("\\\\", "\\")
}

const REMINDER_LABEL: &str = "Reminder";
const TRAVEL_LABEL: &str = "Travel time";

pub fn build_vevent(fields: &VEventFields) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//webcaldav//EN\r\nBEGIN:VEVENT\r\n");
    out.push_str(&format!("UID:{}\r\n", fields.uid));
    out.push_str(&format!("DTSTAMP:{}\r\n", fields.dtstamp));
    if fields.all_day {
        out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", fields.start));
        out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", fields.end));
    } else {
        out.push_str(&format!("DTSTART:{}\r\n", fields.start));
        out.push_str(&format!("DTEND:{}\r\n", fields.end));
    }
    out.push_str(&format!("SUMMARY:{}\r\n", escape_text(fields.summary)));
    if !fields.description.is_empty() {
        out.push_str(&format!("DESCRIPTION:{}\r\n", escape_text(fields.description)));
    }
    if !fields.location.is_empty() {
        out.push_str(&format!("LOCATION:{}\r\n", escape_text(fields.location)));
    }
    if let Some(p) = fields.priority {
        out.push_str(&format!("PRIORITY:{p}\r\n"));
    }
    if let Some(busy) = fields.busy {
        out.push_str(&format!("TRANSP:{}\r\n", if busy { "OPAQUE" } else { "TRANSPARENT" }));
    }
    if let Some(minutes) = fields.reminder_minutes {
        out.push_str("BEGIN:VALARM\r\n");
        out.push_str("ACTION:DISPLAY\r\n");
        out.push_str(&format!("DESCRIPTION:{REMINDER_LABEL}\r\n"));
        out.push_str(&format!("TRIGGER:{}\r\n", format_trigger_minutes(minutes)));
        out.push_str("END:VALARM\r\n");
    }
    if let Some(minutes) = fields.travel_minutes {
        out.push_str("BEGIN:VALARM\r\n");
        out.push_str("ACTION:DISPLAY\r\n");
        out.push_str(&format!("DESCRIPTION:{TRAVEL_LABEL}\r\n"));
        out.push_str(&format!("TRIGGER:{}\r\n", format_trigger_minutes(minutes)));
        out.push_str("END:VALARM\r\n");
    }
    out.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
    out
}

/// Extracts a property's value whether or not it carries parameters, e.g.
/// both `SUMMARY:Title` and `SUMMARY;LANGUAGE=en:Title` yield `Title`.
fn extract_property<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(name)?;
    match rest.chars().next()? {
        ':' => Some(&rest[1..]),
        ';' => rest.find(':').map(|idx| &rest[idx + 1..]),
        _ => None,
    }
}

pub fn parse_vevent(raw: &str) -> Option<ParsedEvent> {
    let mut uid = String::new();
    let mut summary = String::new();
    let mut description = String::new();
    let mut start = String::new();
    let mut end = String::new();
    let mut end_is_date = false;
    let mut location = String::new();
    let mut priority = None;
    let mut busy = None;

    let mut in_alarm = false;
    let mut alarm_desc = String::new();
    let mut alarm_trigger = String::new();
    let mut alarms: Vec<(String, i64)> = Vec::new();

    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line == "BEGIN:VALARM" {
            in_alarm = true;
            alarm_desc.clear();
            alarm_trigger.clear();
            continue;
        }
        if line == "END:VALARM" {
            in_alarm = false;
            if let Some(minutes) = parse_duration_minutes(&alarm_trigger) {
                alarms.push((alarm_desc.clone(), minutes));
            }
            continue;
        }
        if in_alarm {
            if let Some(v) = extract_property(line, "DESCRIPTION") {
                alarm_desc = unescape_text(v);
            } else if let Some(v) = extract_property(line, "TRIGGER") {
                alarm_trigger = v.to_string();
            }
            continue;
        }

        if let Some(v) = extract_property(line, "UID") {
            uid = v.to_string();
        } else if let Some(v) = extract_property(line, "SUMMARY") {
            summary = unescape_text(v);
        } else if let Some(v) = extract_property(line, "DESCRIPTION") {
            description = unescape_text(v);
        } else if let Some(v) = extract_property(line, "LOCATION") {
            location = unescape_text(v);
        } else if let Some(v) = extract_property(line, "PRIORITY") {
            priority = v.trim().parse::<u8>().ok().filter(|p| *p != 0);
        } else if let Some(v) = extract_property(line, "TRANSP") {
            busy = match v.trim() {
                "OPAQUE" => Some(true),
                "TRANSPARENT" => Some(false),
                _ => None,
            };
        } else if line.starts_with("DTEND") {
            end_is_date = line.contains("VALUE=DATE") && !line.contains("VALUE=DATE-TIME");
            if let Some(v) = extract_property(line, "DTEND") {
                end = v.to_string();
            }
        } else if let Some(v) = extract_property(line, "DTSTART") {
            start = v.to_string();
        }
    }

    if uid.is_empty() {
        return None;
    }

    let all_day = start.len() == 8;
    if all_day && end_is_date {
        if let Some(end_date) = parse_yyyymmdd(&end) {
            end = (end_date - Duration::days(1)).format("%Y%m%d").to_string();
        }
    }

    let reminder_minutes = alarms.iter().find(|(d, _)| d == REMINDER_LABEL).map(|(_, m)| *m).or_else(|| {
        alarms.iter().find(|(d, _)| d != TRAVEL_LABEL).map(|(_, m)| *m)
    });
    let travel_minutes = alarms.iter().find(|(d, _)| d == TRAVEL_LABEL).map(|(_, m)| *m);

    Some(ParsedEvent {
        uid,
        summary,
        description,
        start,
        end,
        all_day,
        location,
        priority,
        busy,
        reminder_minutes,
        travel_minutes,
    })
}
