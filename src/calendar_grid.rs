use crate::dav::{self, CalendarSummary, EventSummary, NewEvent};
use crate::layout::{Footer, TopBar};
#[cfg_attr(not(feature = "hydrate"), allow(unused_imports))]
use crate::storage::{self, CalendarPrefs, ConnectionConfig};
use chrono::{Datelike, Duration, NaiveDate};
use leptos::prelude::*;
use crate::ui::{PendingButton, PendingIconButton};
use leptos_router::hooks::use_navigate;
use std::collections::{HashMap, HashSet};

const REMINDER_PRESETS: &[(&str, i64)] = &[("5", 5), ("15", 15), ("30", 30), ("60", 60), ("1440", 1440), ("2880", 2880), ("10080", 10080)];
const TRAVEL_PRESETS: &[(&str, i64)] = &[("0", 0), ("15", 15), ("30", 30), ("45", 45), ("60", 60), ("90", 90)];
/// Seed colours for calendars with no stored preference.
///
/// DESIGN.md's single-accent rule governs chrome, not categorical data colour
/// — four calendars have to be tellable apart, and the colour is user-owned
/// (there is a picker per calendar). Muted so they sit beside Action Blue.
///
/// Referenced only under `hydrate`, so an ssr-only check reports it dead.
/// Deleting it breaks the wasm build.
const COLOR_POOL: &[&str] = &[
    "#0066cc", "#8e5bd0", "#c0567a", "#2f9e6e", "#c2703c", "#4a7fb5", "#9a7b3f", "#6b6f76",
];

/// Single-line form controls. Pill radius per DESIGN.md's `search-input`.
/// 17px is both the system body size and the floor below which iOS zooms on
/// focus.
// A macro, not a const: `concat!` takes literals only, and SELECT below
// has to be built from this same string rather than a copy of it.
macro_rules! field_base {
    () => {
        "w-full h-11 bg-canvas border border-black/[0.08] rounded-full \
    text-body text-ink placeholder:text-ink-48 px-4 \
    focus:outline-none focus:border-action-focus focus:ring-1 focus:ring-action-focus \
    transition-colors"
    };
}
const FIELD: &str = field_base!();

/// A bare `select` keeps the browser's own arrow, which it paints hard
/// against the right edge -- outside the pill's curve, and at a different
/// inset for every control width. `field-select` (lib.rs) suppresses it and
/// draws one chevron at the same 16px inset as the text.
const SELECT: &str = concat!(field_base!(), " field-select");

/// The one control that cannot be a pill: a two-row textarea needs a radius
/// that reads at its corners, so it takes the card radius instead.
const FIELD_AREA: &str = "w-full bg-canvas border border-black/[0.08] rounded-lg \
    text-body text-ink placeholder:text-ink-48 px-4 py-3 resize-none \
    focus:outline-none focus:border-action-focus focus:ring-1 focus:ring-action-focus \
    transition-colors";

fn unit_multiplier(unit: &str) -> i64 {
    match unit {
        "hours" => 60,
        "days" => 1440,
        _ => 1,
    }
}

fn preset_minutes(preset: &str, custom_value: &str, custom_unit: &str) -> Option<i64> {
    match preset {
        "none" | "" => None,
        "custom" => custom_value.trim().parse::<i64>().ok().map(|v| v * unit_multiplier(custom_unit)),
        other => other.parse::<i64>().ok(),
    }
}

fn minutes_to_preset(minutes: Option<i64>, known: &[(&str, i64)]) -> (String, String, String) {
    match minutes {
        None => ("none".to_string(), String::new(), "minutes".to_string()),
        Some(m) => {
            if let Some((label, _)) = known.iter().find(|(_, v)| *v == m) {
                (label.to_string(), String::new(), "minutes".to_string())
            } else {
                ("custom".to_string(), m.to_string(), "minutes".to_string())
            }
        }
    }
}

#[derive(Clone, Copy)]
struct EventForm {
    calendar_url: RwSignal<String>,
    summary: RwSignal<String>,
    description: RwSignal<String>,
    start: RwSignal<String>,
    end: RwSignal<String>,
    all_day: RwSignal<bool>,
    location: RwSignal<String>,
    priority: RwSignal<String>,
    busy: RwSignal<String>,
    reminder_preset: RwSignal<String>,
    reminder_value: RwSignal<String>,
    reminder_unit: RwSignal<String>,
    travel_preset: RwSignal<String>,
    travel_value: RwSignal<String>,
    travel_unit: RwSignal<String>,
}

impl EventForm {
    fn new() -> Self {
        Self {
            calendar_url: RwSignal::new(String::new()),
            summary: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            start: RwSignal::new(String::new()),
            end: RwSignal::new(String::new()),
            all_day: RwSignal::new(false),
            location: RwSignal::new(String::new()),
            priority: RwSignal::new(String::new()),
            busy: RwSignal::new(String::new()),
            reminder_preset: RwSignal::new("none".to_string()),
            reminder_value: RwSignal::new(String::new()),
            reminder_unit: RwSignal::new("minutes".to_string()),
            travel_preset: RwSignal::new("none".to_string()),
            travel_value: RwSignal::new(String::new()),
            travel_unit: RwSignal::new("minutes".to_string()),
        }
    }

    fn clear(&self) {
        self.summary.set(String::new());
        self.description.set(String::new());
        self.start.set(String::new());
        self.end.set(String::new());
        self.all_day.set(false);
        self.location.set(String::new());
        self.priority.set(String::new());
        self.busy.set(String::new());
        self.reminder_preset.set("none".to_string());
        self.reminder_value.set(String::new());
        self.reminder_unit.set("minutes".to_string());
        self.travel_preset.set("none".to_string());
        self.travel_value.set(String::new());
        self.travel_unit.set("minutes".to_string());
    }

    fn fill_from(&self, event: &EventSummary) {
        self.calendar_url.set(event.calendar_url.clone());
        self.summary.set(event.summary.clone());
        self.description.set(event.description.clone());
        self.start.set(event.start.clone());
        self.end.set(event.end.clone());
        self.all_day.set(event.all_day);
        self.location.set(event.location.clone());
        self.priority.set(event.priority.map(|p| p.to_string()).unwrap_or_default());
        self.busy.set(match event.busy {
            Some(true) => "busy".to_string(),
            Some(false) => "free".to_string(),
            None => String::new(),
        });
        let (rp, rv, ru) = minutes_to_preset(event.reminder_minutes, REMINDER_PRESETS);
        self.reminder_preset.set(rp);
        self.reminder_value.set(rv);
        self.reminder_unit.set(ru);
        let (tp, tv, tu) = minutes_to_preset(event.travel_minutes, TRAVEL_PRESETS);
        self.travel_preset.set(tp);
        self.travel_value.set(tv);
        self.travel_unit.set(tu);
    }

    fn set_default_day(&self, day: NaiveDate) {
        self.start.set(format!("{}T09:00", day.format("%Y-%m-%d")));
        self.end.set(format!("{}T10:00", day.format("%Y-%m-%d")));
    }

    fn toggle_all_day(&self) {
        let now_all_day = !self.all_day.get_untracked();
        self.all_day.set(now_all_day);
        if now_all_day {
            self.start.update(|s| s.truncate(10));
            self.end.update(|s| s.truncate(10));
        } else {
            self.start.update(|s| {
                if s.len() == 10 {
                    s.push_str("T09:00");
                }
            });
            self.end.update(|s| {
                if s.len() == 10 {
                    s.push_str("T10:00");
                }
            });
        }
    }

    fn to_new_event(&self) -> NewEvent {
        NewEvent {
            summary: self.summary.get_untracked(),
            description: self.description.get_untracked(),
            start: self.start.get_untracked(),
            end: self.end.get_untracked(),
            all_day: self.all_day.get_untracked(),
            location: self.location.get_untracked(),
            priority: self.priority.get_untracked().parse::<u8>().ok(),
            busy: match self.busy.get_untracked().as_str() {
                "busy" => Some(true),
                "free" => Some(false),
                _ => None,
            },
            reminder_minutes: preset_minutes(&self.reminder_preset.get_untracked(), &self.reminder_value.get_untracked(), &self.reminder_unit.get_untracked()),
            travel_minutes: preset_minutes(&self.travel_preset.get_untracked(), &self.travel_value.get_untracked(), &self.travel_unit.get_untracked()),
        }
    }
}

#[cfg(feature = "hydrate")]
fn random_fill(form: EventForm) {
    use fake::faker::address::en::CityName;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;

    fn pick(options: &[&str]) -> String {
        let idx = ((js_sys::Math::random() * options.len() as f64) as usize).min(options.len() - 1);
        options[idx].to_string()
    }

    let summary: String = Sentence(3..7).fake();
    form.summary.set(summary.trim_end_matches('.').to_string());

    let description: String = Paragraph(1..3).fake();
    form.description.set(description);

    let location: String = CityName().fake();
    form.location.set(location);

    form.priority.set(pick(&["", "1", "5", "9"]));
    form.busy.set(pick(&["", "busy", "free"]));
    form.reminder_preset.set(pick(&["none", "5", "15", "30", "60", "1440"]));
    form.reminder_value.set(String::new());
    form.reminder_unit.set("minutes".to_string());
    form.travel_preset.set(pick(&["none", "0", "15", "30", "45", "60"]));
    form.travel_value.set(String::new());
    form.travel_unit.set("minutes".to_string());

    let date_part: String = form.start.get_untracked().chars().take(10).collect();
    if date_part.len() == 10 {
        if form.all_day.get_untracked() {
            form.start.set(date_part.clone());
            form.end.set(date_part);
        } else {
            let hour = 8 + (js_sys::Math::random() * 10.0) as u32;
            form.start.set(format!("{date_part}T{hour:02}:00"));
            form.end.set(format!("{date_part}T{:02}:00", hour + 1));
        }
    }
}

#[cfg(not(feature = "hydrate"))]
fn random_fill(_form: EventForm) {}

fn shift_month(date: NaiveDate, delta: i32) -> NaiveDate {
    let mut year = date.year();
    let mut month = date.month() as i32 + delta;
    while month < 1 {
        month += 12;
        year -= 1;
    }
    while month > 12 {
        month -= 12;
        year += 1;
    }
    NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap()
}

fn days_in_month(first_of_month: NaiveDate) -> i64 {
    let next = shift_month(first_of_month, 1);
    (next - first_of_month).num_days()
}

fn grid_cells(first_of_month: NaiveDate) -> Vec<(NaiveDate, bool)> {
    let leading = first_of_month.weekday().num_days_from_monday() as i64;
    let total_days = days_in_month(first_of_month);
    let last_of_month = first_of_month + Duration::days(total_days - 1);
    let raw_len = leading + total_days;
    let grid_len = raw_len + ((7 - raw_len % 7) % 7);
    let start = first_of_month - Duration::days(leading);
    (0..grid_len)
        .map(|i| {
            let d = start + Duration::days(i);
            (d, d >= first_of_month && d <= last_of_month)
        })
        .collect()
}

/// How busy a day is. Reads out in the day header, not the month cell — a
/// ~53px cell fits a date and some dots and nothing more.
fn slot_label(count: usize) -> String {
    match count {
        0 => "No events".to_string(),
        1 => "1 event".to_string(),
        n => format!("{n} events"),
    }
}

#[component]
pub fn CalendarGridRoutePage() -> impl IntoView {
    #[cfg_attr(not(feature = "hydrate"), allow(unused_variables))]
    let navigate = use_navigate();

    let username = RwSignal::new(String::new());
    let config = RwSignal::new(Option::<ConnectionConfig>::None);

    let calendars = RwSignal::new(Vec::<CalendarSummary>::new());
    let calendar_colors = RwSignal::new(HashMap::<String, String>::new());
    let hidden_calendars = RwSignal::new(HashSet::<String>::new());

    let events = RwSignal::new(Vec::<EventSummary>::new());
    let status = RwSignal::new(String::new());
    let latency_ms = RwSignal::new(Option::<i64>::None);
    // A timestamp, not a formatted string: the "is this stale" test on
    // tab focus needs to subtract it, and the display can format it.
    let last_synced = RwSignal::new(Option::<chrono::DateTime<chrono::Local>>::None);

    let today = chrono::Local::now().date_naive();
    let current_month = RwSignal::new(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap());
    let selected_day = RwSignal::new(Option::<NaiveDate>::None);

    let editing_event = RwSignal::new(Option::<EventSummary>::None);
    // The form lived in a desktop sidebar and was always visible, which left
    // a button sitting on top of an open form once the columns collapsed.
    // In one column it has to be summoned.
    let form_open = RwSignal::new(false);
    let month_picker_open = RwSignal::new(false);
    let form = EventForm::new();

    let calendar_color = move |calendar_url: &str| -> String {
        calendar_colors.get().get(calendar_url).cloned().unwrap_or_else(|| COLOR_POOL[0].to_string())
    };

    // Every network call on this page is an Action. `.pending()` is derived
    // from an in-flight counter the runtime maintains, so a flag can no
    // longer be left stuck by an error path, and the button bound to it is
    // reading the request's real state rather than a copy of it.
    let refresh = Action::new_local(move |_: &()| async move {
        let Some(cfg) = config.get_untracked() else { return };
        let urls: Vec<String> = calendars.get_untracked().iter().map(|c| c.url.clone()).collect();
        if urls.is_empty() {
            return;
        }
        match dav::list_events_for_calendars(&cfg, &urls).await {
            Ok(fetch) => {
                events.set(fetch.events);
                latency_ms.set(Some(fetch.latency_ms));
                last_synced.set(Some(chrono::Local::now()));
                status.set(String::new());
            }
            Err(e) => status.set(format!("Failed to load events: {e}")),
        }
    });
    let refreshing = refresh.pending();
    // Dispatching twice really does run twice. Four callers -- mount, save,
    // delete, tab focus -- can collide, and the slower response would win
    // regardless of which request was newer.
    let refresh_events = move || {
        if !refreshing.get_untracked() {
            refresh.dispatch(());
        }
    };

    // Opening the page is itself a request, so it is an action too -- that is
    // what keeps "Updating…" on screen from first paint instead of appearing
    // only once the second call starts.
    let bootstrap = Action::new_local(move |cfg: &ConnectionConfig| {
        let cfg = cfg.clone();
        async move {
            match dav::list_calendars(&cfg).await {
                Ok(found) => {
                    let mut prefs = storage::load_calendar_prefs();
                    let mut changed = false;
                    for (i, cal) in found.iter().enumerate() {
                        if !prefs.colors.contains_key(&cal.url) {
                            prefs.colors.insert(cal.url.clone(), COLOR_POOL[i % COLOR_POOL.len()].to_string());
                            changed = true;
                        }
                    }
                    if changed {
                        storage::save_calendar_prefs(&prefs);
                    }
                    calendar_colors.set(prefs.colors);
                    hidden_calendars.set(prefs.hidden.into_iter().filter(|(_, hidden)| *hidden).map(|(url, _)| url).collect());
                    calendars.set(found);
                    refresh_events();
                }
                Err(e) => status.set(format!("Failed to load calendars: {e}")),
            }
        }
    });
    let booting = bootstrap.pending();
    // False on the server and on the client's first render, so the two agree
    // and hydration has nothing to reconcile. Flipped once the effect below
    // has actually run.
    let hydrated = RwSignal::new(false);
    // One flag for "a read is in flight", whichever of the two it is -- plus
    // the window before any of them can have started. Without that last term
    // the server renders a finished-looking empty calendar and the first sign
    // of life is the username appearing ~2s later: measured 566ms to painted
    // HTML, 2498ms to username, on a desktop with a good connection.
    let loading = Signal::derive(move || !hydrated.get() || booting.get() || refreshing.get());

    #[cfg(feature = "hydrate")]
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let Some(cfg) = storage::load() else {
                navigate("/", Default::default());
                return;
            };
            hydrated.set(true);
            username.set(cfg.username.clone());
            config.set(Some(cfg.clone()));
            bootstrap.dispatch(cfg.clone());
        });
    }


    // Nothing here polls, so returning to the tab is the only moment the app
    // can notice someone changed the calendar from another device. Apple
    // Calendar, Thunderbird and DAVx5 all refresh on foreground for the same
    // reason -- they just also poll on a timer, which this does not yet.
    #[cfg(feature = "hydrate")]
    {
        // A full fetch pulls every event in every calendar, so an alt-tabbing
        // user must not trigger one per switch. Below the floor the data is
        // young enough that the round trip buys nothing.
        const REFRESH_FLOOR_SECS: i64 = 60;
        let handle = window_event_listener_untyped(
            "visibilitychange",
            move |_| {
                if document().hidden() {
                    return;
                }
                let stale = last_synced.get_untracked().map_or(true, |t| {
                    (chrono::Local::now() - t).num_seconds() >= REFRESH_FLOOR_SECS
                });
                if stale {
                    refresh_events();
                }
            },
        );
        on_cleanup(move || handle.remove());
    }

    let persist_prefs = move || {
        let prefs = CalendarPrefs {
            colors: calendar_colors.get_untracked(),
            hidden: hidden_calendars.get_untracked().into_iter().map(|url| (url, true)).collect(),
        };
        storage::save_calendar_prefs(&prefs);
    };

    let toggle_calendar_visible = move |url: String| {
        hidden_calendars.update(|set| {
            if !set.remove(&url) {
                set.insert(url);
            }
        });
        persist_prefs();
    };

    let set_calendar_color = move |url: String, color: String| {
        calendar_colors.update(|map| {
            map.insert(url, color);
        });
        persist_prefs();
    };

    let first_visible_calendar = move || {
        calendars.get_untracked().iter().map(|c| c.url.clone()).find(|url| !hidden_calendars.get_untracked().contains(url)).unwrap_or_default()
    };

    // Only the selection. The four lines that used to follow -- clearing the
    // form, resetting the calendar, re-dating it -- are all repeated verbatim
    // by `open_new_event`, so doing them here bought nothing and cost two
    // things: about twenty signal writes on every tap of a date, and a form
    // that wiped whatever had been typed into it the moment the user picked a
    // different day while it was open.
    let select_day = move |day: NaiveDate| {
        selected_day.set(Some(day));
    };

    let save = Action::new_local(move |_: &()| {
        // Read the form at dispatch time, not inside the future: by the time
        // it resolves the user may already be typing the next event.
        let cfg = config.get_untracked();
        let target_calendar = form.calendar_url.get_untracked();
        let new_event = form.to_new_event();
        let editing = editing_event.get_untracked();
        async move {
            let Some(cfg) = cfg else { return };
            let result = match editing {
                Some(existing) => dav::update_event(&cfg, &existing, &target_calendar, new_event).await,
                None => dav::create_event(&cfg, &target_calendar, new_event).await,
            };
            match result {
                Ok(()) => {
                    editing_event.set(None);
                    form.clear();
                    // Closes only on success. A failed save leaves the form
                    // open with what was typed still in it, rather than
                    // dismissing the sheet and the work together.
                    form_open.set(false);
                    refresh_events();
                }
                Err(e) => status.set(format!("Save failed: {e}")),
            }
        }
    });
    let saving = save.pending();
    let submit_event = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        save.dispatch(());
    };

    let open_new_event = move |_| {
        let day = selected_day.get_untracked().unwrap_or(today);
        selected_day.set(Some(day));
        editing_event.set(None);
        form.clear();
        form.calendar_url.set(first_visible_calendar());
        form.set_default_day(day);
        form_open.set(true);
    };

    let start_edit = move |event: EventSummary| {
        form.fill_from(&event);
        editing_event.set(Some(event));
        form_open.set(true);
    };

    let cancel_edit = move |_| {
        editing_event.set(None);
        form.clear();
        form_open.set(false);
    };

    // The input is the event itself, which is what lets a single action
    // drive a per-row spinner: the view compares `remove.input()` against its
    // own event instead of every row reacting to one shared flag.
    let remove = Action::new_local(move |event: &EventSummary| {
        let event = event.clone();
        let cfg = config.get_untracked();
        async move {
            let Some(cfg) = cfg else { return };
            match dav::delete_event(&cfg, &event).await {
                Ok(()) => refresh_events(),
                Err(e) => status.set(format!("Delete failed: {e}")),
            }
        }
    });
    let removing = remove.pending();
    let remove_input = remove.input();

    let events_on = move |day: NaiveDate| -> Vec<EventSummary> {
        let prefix = day.format("%Y-%m-%d").to_string();
        let hidden = hidden_calendars.get();
        events.get().into_iter().filter(|e| e.start.starts_with(&prefix) && !hidden.contains(&e.calendar_url)).collect()
    };

    view! {
        <TopBar username=username/>

        // One column, phone-shaped. The background is painted on <body>, so a
        // desktop viewport simply shows more parchment either side — there is
        // no second layout to keep in step with this one.
        <main class="w-full flex-1">
            <div class="max-w-app mx-auto w-full px-4 py-5 flex flex-col gap-5">

                <Show when=move || !status.get().is_empty()>
                    // Errors read as errors through wording and placement. The
                    // old orange panel was a second accent, which DESIGN.md
                    // rules out outright.
                    // role="alert" carries an implicit assertive live region,
                    // so inserting this node is what announces it. Without one
                    // a failed save is silent to a screen reader on every path.
                    <div
                        role="alert"
                        data-testid="status-error"
                        class="flex items-start gap-2.5 p-4 rounded-lg bg-canvas border border-hairline"
                    >
                        <span class="material-symbols-outlined text-[20px] text-ink-48 shrink-0">
                            "error_outline"
                        </span>
                        <span class="text-caption text-ink-80">{move || status.get()}</span>
                    </div>
                </Show>

                // Month navigation sits directly on the parchment. DESIGN.md
                // takes its section breaks from surface-colour change rather
                // than chrome, so the card below does the dividing and this
                // needs no border of its own.
                <div class="flex flex-col gap-3">
                    <div class="flex items-center justify-between gap-2">
                        // Tapping the title opens the picker, as iOS
                        // Calendar does. The chevrons stay for stepping a
                        // month at a time; the picker is for jumping.
                        // An h2 so the page can be skimmed by heading. Tailwind's
                        // preflight zeroes heading margins and inherits the size,
                        // so this changes nothing visually.
                        <h2 class="min-w-0">
                        <button
                            type="button"
                            data-testid="month-title"
                            aria-controls="month-picker"
                            class="flex items-center gap-1 min-w-0 h-11 text-tagline font-semibold text-ink"
                            aria-expanded=move || month_picker_open.get().to_string()
                            on:click=move |_| month_picker_open.update(|v| *v = !*v)
                        >
                            <span class="truncate">
                                {move || current_month.get().format("%B %Y").to_string()}
                            </span>
                            <span class="material-symbols-outlined text-[20px] text-action shrink-0">
                                {move || if month_picker_open.get() { "expand_less" } else { "expand_more" } }
                            </span>
                        </button>
                        </h2>
                        <div class="flex items-center shrink-0">
                            <button
                                data-testid="today-button"
                                type="button"
                                class="h-11 px-3 text-caption font-semibold text-action"
                                on:click=move |_| current_month.set(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap())
                            >
                                "Today"
                            </button>
                            // 44px squares: the spec's circular control size,
                            // and the platform's minimum comfortable target.
                            <button
                                type="button"
                                class="w-11 h-11 flex items-center justify-center rounded-full text-action"
                                data-testid="month-prev"
                                aria-label="Previous month"
                                on:click=move |_| current_month.update(|d| *d = shift_month(*d, -1))
                            >
                                <span class="material-symbols-outlined text-[22px]">"chevron_left"</span>
                            </button>
                            <button
                                type="button"
                                class="w-11 h-11 flex items-center justify-center rounded-full text-action"
                                data-testid="month-next"
                                aria-label="Next month"
                                on:click=move |_| current_month.update(|d| *d = shift_month(*d, 1))
                            >
                                <span class="material-symbols-outlined text-[22px]">"chevron_right"</span>
                            </button>
                        </div>
                    </div>

                    // Two native <select>s rather than a bespoke wheel: iOS
                    // renders them as the system picker, which is the control
                    // the user already knows, and it costs no custom UI.
                    // The wrapper is always rendered: aria-controls above must
                    // point at something that exists even while it is closed.
                    <div id="month-picker">
                    <Show when=move || month_picker_open.get()>
                        <div class="flex items-center gap-2">
                            <select
                                class=SELECT
                                data-testid="month-select"
                                aria-label="Month"
                                prop:value=move || current_month.get().month().to_string()
                                on:change=move |ev| {
                                    if let Ok(m) = event_target_value(&ev).parse::<u32>() {
                                        current_month
                                            .update(|d| {
                                                if let Some(next) = NaiveDate::from_ymd_opt(d.year(), m, 1) {
                                                    *d = next;
                                                }
                                            });
                                    }
                                }
                            >
                                {(1..=12u32)
                                    .map(|m| {
                                        let label = NaiveDate::from_ymd_opt(2000, m, 1)
                                            .unwrap()
                                            .format("%B")
                                            .to_string();
                                        view! { <option value=m.to_string()>{label}</option> }
                                    })
                                    .collect_view()}
                            </select>
                            <select
                                class=SELECT
                                data-testid="year-select"
                                aria-label="Year"
                                prop:value=move || current_month.get().year().to_string()
                                on:change=move |ev| {
                                    if let Ok(y) = event_target_value(&ev).parse::<i32>() {
                                        current_month
                                            .update(|d| {
                                                if let Some(next) = NaiveDate::from_ymd_opt(y, d.month(), 1) {
                                                    *d = next;
                                                }
                                            });
                                    }
                                }
                            >
                                // Ten years either side of today. The chevrons
                                // cover anything beyond that, and a longer list
                                // makes the wheel worse to spin.
                                {((today.year() - 10)..=(today.year() + 10))
                                    .map(|y| view! { <option value=y.to_string()>{y.to_string()}</option> })
                                    .collect_view()}
                            </select>
                        </div>
                    </Show>
                    </div>

                    // `button-primary`: Action Blue, full pill, body text. The
                    // press-scale is global in lib.rs so it cannot be left off.
                    <button
                        type="button"
                        data-testid="add-event"
                        class="w-full h-12 rounded-full bg-action text-canvas text-body flex items-center justify-center gap-2"
                        on:click=open_new_event
                    >
                        <span class="material-symbols-outlined text-[20px]">"add"</span>
                        "Add event"
                    </button>
                </div>

                <div class="flex flex-col gap-2">
                    // Uniform, weekends included. The old grid coloured Saturday
                    // and Sunday orange; a second accent is not available, and
                    // iOS Calendar does not distinguish them either.
                    <div class="grid grid-cols-7 gap-1 px-1">
                        {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                            .iter()
                            .map(|d| {
                                view! {
                                    <div class="text-fine text-ink-48 text-center py-1">{*d}</div>
                                }
                            })
                            .collect_view()}
                    </div>

                    <div class="bg-canvas border border-hairline rounded-lg p-2">
                        <div class="grid grid-cols-7 gap-1">
                            {move || {
                                grid_cells(current_month.get())
                                    .into_iter()
                                    .map(|(day, in_month)| {
                                        let is_today = day == today;
                                        let is_selected = move || selected_day.get() == Some(day);
                                        let day_events = events_on(day);
                                        view! {
                                            // A real <button>: keyboard
                                            // reachable and it announces
                                            // itself.
                                            <button
                                                type="button"
                                                data-testid="day-cell"
                                                // Without this the whole
                                                // accessible name is "25".
                                                // Today, the event count and
                                                // the selection are things the
                                                // design says with a filled
                                                // disc, a blue number and
                                                // coloured dots -- none of
                                                // which is readable aloud.
                                                aria-label=format!(
                                                    "{}{}{}",
                                                    day.format("%A %-d %B %Y"),
                                                    if is_today { ", today" } else { "" },
                                                    match day_events.len() {
                                                        0 => String::new(),
                                                        1 => ", 1 event".to_string(),
                                                        n => format!(", {n} events"),
                                                    },
                                                )
                                                aria-current=is_today.then_some("date")
                                                aria-pressed=move || is_selected().to_string()
                                                // Addressable by date, so a
                                                // grid assertion names the day
                                                // it means instead of counting
                                                // squares from a corner.
                                                data-date=day.format("%Y-%m-%d").to_string()
                                                data-in-month=in_month.to_string()
                                                data-today=is_today.to_string()
                                                data-selected=move || is_selected().to_string()
                                                data-event-count=day_events.len().to_string()
                                                class="aspect-square rounded-sm flex flex-col items-center justify-center gap-1"
                                                on:click=move |_| select_day(day)
                                            >
                                                // Selection gets the filled
                                                // disc, today only blue text.
                                                // Reversed once and the
                                                // selection lost: two filled
                                                // discs competed and the one
                                                // marking a fact beat the one
                                                // marking the user's own tap.
                                                // 36px in a ~53px cell.
                                                <span
                                                    class="w-9 h-9 flex items-center justify-center rounded-full text-caption"
                                                    class=("bg-action", move || is_selected())
                                                    class=("text-canvas", move || is_selected())
                                                    class=("font-semibold", move || is_selected() || is_today)
                                                    class=("text-action", move || is_today && !is_selected())
                                                    class=("text-ink", move || in_month && !is_today && !is_selected())
                                                    // Dimmed by colour, not
                                                    // opacity. `opacity-40` on
                                                    // #1d1d1f fails AA contrast
                                                    // at 1.8:1, and these cells
                                                    // are still tappable, so it
                                                    // is interactive text that
                                                    // cannot be read.
                                                    class=("text-ink-48", move || !in_month && !is_today && !is_selected())
                                                >
                                                    {day.day()}
                                                </span>
                                                // Fixed height whether or not
                                                // there are dots, so the date
                                                // row does not shift between
                                                // busy and empty days.
                                                // The dots keep the dimming:
                                                // they carry no text, so no
                                                // contrast rule applies.
                                                <span
                                                    data-testid="day-dots"
                                                    class="flex items-center gap-0.5 h-1.5"
                                                    class=("opacity-40", move || !in_month)
                                                >
                                                    {day_events
                                                        .iter()
                                                        .take(3)
                                                        .map(|e| {
                                                            let dot = calendar_color(&e.calendar_url);
                                                            view! {
                                                                <span
                                                                    class="w-1.5 h-1.5 rounded-full"
                                                                    style:background-color=dot
                                                                ></span>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </span>
                                            </button>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </div>

                    // Kept because it answers "am I looking at stale data".
                    // The latency readout that used to sit beside it answered
                    // nothing a calendar user can act on.
                    <Show when=move || last_synced.get().is_some() || loading.get()>
                      // Polite, not assertive: "Updated 14:32" should wait for
                      // a gap rather than interrupt.
                      <div
                        data-testid="sync-line"
                        aria-live="polite"
                        class="flex items-center justify-center gap-1"
                      >
                        // Same indicator as the buttons use, so "working" looks
                        // the same wherever it happens.
                        <Show when=move || loading.get()>
                            <crate::ui::Spinner class="text-fine text-ink-48 mr-1"/>
                        </Show>
                        <p class="text-fine text-ink-48">
                            {move || {
                                if loading.get() {
                                    "Updating…".to_string()
                                } else {
                                    last_synced
                                        .get()
                                        .map(|t| format!("Updated {}", t.format("%H:%M")))
                                        .unwrap_or_default()
                                }
                            }}
                        </p>
                        // Sits with the staleness it answers rather than in the
                        // top bar: asking "is this current" and fixing it are
                        // the same thought. Skips the focus floor -- an
                        // explicit tap is not an incidental tab switch.
                        <button
                            type="button"
                            data-testid="refresh"
                            class="h-11 px-2 flex items-center text-fine font-semibold text-action disabled:text-ink-48"
                            disabled=move || loading.get()
                            on:click=move |_| refresh_events()
                        >
                            "Refresh"
                        </button>
                      </div>
                    </Show>
                </div>

                <div
                    data-testid="calendars-panel"
                    class="bg-canvas border border-hairline rounded-lg p-4 flex flex-col gap-2"
                >
                    <h2 class="text-caption font-semibold text-ink">"Calendars"</h2>
                    <For
                        each=move || calendars.get()
                        key=|c| c.url.clone()
                        children=move |cal: CalendarSummary| {
                            let url_for_toggle = cal.url.clone();
                            let url_for_visible = cal.url.clone();
                            let url_for_color_get = cal.url.clone();
                            let url_for_color_set = cal.url.clone();
                            view! {
                                <label
                                    data-testid="calendar-row"
                                    data-url=cal.url.clone()
                                    class="flex items-center gap-3 min-h-11 cursor-pointer"
                                >
                                    <input
                                        type="checkbox"
                                        data-testid="calendar-visible"
                                        class="w-5 h-5 accent-action shrink-0"
                                        prop:checked=move || !hidden_calendars.get().contains(&url_for_visible)
                                        on:change=move |_| toggle_calendar_visible(url_for_toggle.clone())
                                    />
                                    <input
                                        type="color"
                                        data-testid="calendar-color"
                                        class="w-7 h-7 rounded-full border-none bg-transparent cursor-pointer shrink-0"
                                        prop:value=move || calendar_color(&url_for_color_get)
                                        on:input=move |ev| set_calendar_color(url_for_color_set.clone(), event_target_value(&ev))
                                    />
                                    <span class="text-body text-ink truncate">
                                        {cal.display_name.clone()}
                                    </span>
                                </label>
                            }
                        }
                    />
                </div>

                <div class="bg-canvas border border-hairline rounded-lg overflow-hidden">
                    <Show
                        when=move || selected_day.get().is_some()
                        fallback=|| view! {
                            <p data-testid="day-detail-empty" class="text-body text-ink-80 p-6">
                                "Pick a day to see what is on it."
                            </p>
                        }
                    >
                        <div
                            data-testid="day-detail"
                            class="px-4 py-3 border-b border-divider-soft flex flex-col gap-0.5"
                        >
                            <h2 data-testid="day-detail-date" class="text-body font-semibold text-ink">
                                {move || selected_day.get().map(|d| d.format("%A %-d %B").to_string()).unwrap_or_default()}
                            </h2>
                            <span data-testid="day-detail-count" class="text-caption text-ink-48">
                                {move || slot_label(selected_day.get().map(events_on).unwrap_or_default().len())}
                            </span>
                        </div>

                        <div class="flex flex-col gap-2 p-4">
                            <For
                                each=move || selected_day.get().map(events_on).unwrap_or_default()
                                key=|e| e.uid.clone()
                                children=move |event: EventSummary| {
                                    let event_for_edit = event.clone();
                                    let event_for_delete = event.clone();
                                    let dot = calendar_color(&event.calendar_url);
                                    view! {
                                        <div
                                            data-testid="event-card"
                                            data-uid=event.uid.clone()
                                            class="rounded-lg border border-hairline p-3 flex items-start justify-between gap-2"
                                        >
                                            <div class="flex flex-col gap-1 min-w-0 flex-1">
                                                <span data-testid="event-time" class="text-caption text-ink-48">
                                                    {event.start.clone()}" – "{event.end.clone()}
                                                </span>
                                                <span class="flex items-center gap-2 min-w-0">
                                                    <span
                                                        class="w-2 h-2 rounded-full shrink-0"
                                                        style:background-color=dot
                                                    ></span>
                                                    <span data-testid="event-summary" class="text-body font-semibold text-ink truncate">
                                                        {event.summary.clone()}
                                                    </span>
                                                </span>
                                                <Show when={
                                                    let d = event.description.clone();
                                                    move || !d.is_empty()
                                                }>
                                                    <span data-testid="event-description" class="text-caption text-ink-80">
                                                        {event.description.clone()}
                                                    </span>
                                                </Show>
                                            </div>
                                            <div class="flex items-center shrink-0">
                                                <button
                                                    type="button"
                                                    class="w-11 h-11 flex items-center justify-center rounded-full text-action"
                                                    data-testid="event-edit"
                                                    aria-label="Edit event"
                                                    on:click=move |_| start_edit(event_for_edit.clone())
                                                >
                                                    <span class="material-symbols-outlined text-[18px]">"edit"</span>
                                                </button>
                                                // Destructive, but it does not
                                                // get a red: the system has one
                                                // accent, and the glyph already
                                                // carries the intent.
                                                <PendingIconButton
                                                    // Narrowed to this row: one
                                                    // action serves the whole
                                                    // list, so without the input
                                                    // comparison every row would
                                                    // spin on one tap.
                                                    pending=Signal::derive({
                                                        let mine = event_for_delete.clone();
                                                        move || removing.get()
                                                            && remove_input.get().as_ref() == Some(&mine)
                                                    })
                                                    icon="delete"
                                                    label="Delete event"
                                                    testid="event-delete"
                                                    class="w-11 h-11 flex items-center justify-center rounded-full text-ink-48"
                                                    on_click=Callback::new({
                                                        let mine = event_for_delete.clone();
                                                        move |_| { remove.dispatch(mine.clone()); }
                                                    })
                                                />
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>

                        <Show when=move || form_open.get()>
                        <div class="px-4 pb-4 pt-2 border-t border-divider-soft flex flex-col gap-4">
                            <div class="flex items-center justify-between gap-2">
                                <span class="text-caption font-semibold text-ink">
                                    {move || if editing_event.get().is_some() { "Edit event" } else { "New event" }}
                                </span>
                                <button
                                    type="button"
                                    data-testid="form-random"
                                    class="h-11 px-2 flex items-center gap-1.5 text-caption text-ink-48"
                                    title="Fill with random test data"
                                    on:click=move |_| random_fill(form)
                                >
                                    <span class="material-symbols-outlined text-[16px]">"casino"</span>
                                    "Random"
                                </button>
                            </div>

                            <form
                                data-testid="event-form"
                                on:submit=submit_event
                                class="flex flex-col gap-3"
                            >
                                <select
                                    data-testid="form-calendar-url"
                                    aria-label="Calendar"
                                    class=SELECT
                                    prop:value=move || form.calendar_url.get()
                                    on:change=move |ev| form.calendar_url.set(event_target_value(&ev))
                                >
                                    {move || calendars.get().into_iter().map(|c| view! {
                                        <option value=c.url.clone()>{c.display_name.clone()}</option>
                                    }).collect_view()}
                                </select>
                                <input
                                    data-testid="form-summary"
                                    class=FIELD
                                    type="text"
                                    placeholder="Title"
                                    prop:value=move || form.summary.get()
                                    on:input=move |ev| form.summary.set(event_target_value(&ev))
                                    required
                                />
                                <textarea
                                    data-testid="form-description"
                                    class=FIELD_AREA
                                    rows="2"
                                    placeholder="Notes"
                                    prop:value=move || form.description.get()
                                    on:input=move |ev| form.description.set(event_target_value(&ev))
                                ></textarea>
                                <label class="flex items-center gap-3 min-h-11 cursor-pointer">
                                    <input
                                        data-testid="form-all-day"
                                        type="checkbox"
                                        class="w-5 h-5 accent-action shrink-0"
                                        prop:checked=move || form.all_day.get()
                                        on:change=move |_| form.toggle_all_day()
                                    />
                                    <span class="text-body text-ink">"All day"</span>
                                </label>
                                // One per row rather than side by side: a datetime-local renders its
                                // own date *and* time text, which clips at half of a 430px column.
                                // Stacked they need labels -- two identical pills say nothing about
                                // which is which.
                                <label class="flex flex-col gap-1.5">
                                    <span class="text-caption text-ink-48 px-1">"Starts"</span>
                                    <input
                                        data-testid="form-start"
                                        class=FIELD
                                        type=move || if form.all_day.get() { "date" } else { "datetime-local" }
                                        prop:value=move || form.start.get()
                                        on:input=move |ev| form.start.set(event_target_value(&ev))
                                        required
                                    />
                                </label>
                                <label class="flex flex-col gap-1.5">
                                    <span class="text-caption text-ink-48 px-1">"Ends"</span>
                                    <input
                                        data-testid="form-end"
                                        class=FIELD
                                        type=move || if form.all_day.get() { "date" } else { "datetime-local" }
                                        prop:value=move || form.end.get()
                                        on:input=move |ev| form.end.set(event_target_value(&ev))
                                        required
                                    />
                                </label>
                                <input
                                    data-testid="form-location"
                                    class=FIELD
                                    type="text"
                                    placeholder="Location"
                                    prop:value=move || form.location.get()
                                    on:input=move |ev| form.location.set(event_target_value(&ev))
                                />
                                <div class="grid grid-cols-2 gap-2">
                                    <select
                                        data-testid="form-priority"
                                        aria-label="Priority"
                                        class=SELECT
                                        prop:value=move || form.priority.get()
                                        on:change=move |ev| form.priority.set(event_target_value(&ev))
                                    >
                                        <option value="">"Priority"</option>
                                        <option value="1">"High"</option>
                                        <option value="5">"Medium"</option>
                                        <option value="9">"Low"</option>
                                    </select>
                                    <select
                                        data-testid="form-busy"
                                        aria-label="Availability"
                                        class=SELECT
                                        prop:value=move || form.busy.get()
                                        on:change=move |ev| form.busy.set(event_target_value(&ev))
                                    >
                                        <option value="">"Status"</option>
                                        <option value="busy">"Busy"</option>
                                        <option value="free">"Free"</option>
                                    </select>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <select
                                        data-testid="form-reminder-preset"
                                        aria-label="Reminder"
                                        class=SELECT
                                        prop:value=move || form.reminder_preset.get()
                                        on:change=move |ev| form.reminder_preset.set(event_target_value(&ev))
                                    >
                                        <option value="none">"No reminder"</option>
                                        <option value="5">"5 minutes before"</option>
                                        <option value="15">"15 minutes before"</option>
                                        <option value="30">"30 minutes before"</option>
                                        <option value="60">"1 hour before"</option>
                                        <option value="1440">"1 day before"</option>
                                        <option value="2880">"2 days before"</option>
                                        <option value="10080">"1 week before"</option>
                                        <option value="custom">"Custom…"</option>
                                    </select>
                                    <Show when=move || form.reminder_preset.get() == "custom">
                                        <div class="grid grid-cols-2 gap-2">
                                            <input
                                                data-testid="form-reminder-value"
                                                class=FIELD
                                                type="number"
                                                min="0"
                                                placeholder="Value"
                                                prop:value=move || form.reminder_value.get()
                                                on:input=move |ev| form.reminder_value.set(event_target_value(&ev))
                                            />
                                            <select
                                                data-testid="form-reminder-unit"
                                                aria-label="Reminder unit"
                                                class=SELECT
                                                prop:value=move || form.reminder_unit.get()
                                                on:change=move |ev| form.reminder_unit.set(event_target_value(&ev))
                                            >
                                                <option value="minutes">"minutes"</option>
                                                <option value="hours">"hours"</option>
                                                <option value="days">"days"</option>
                                            </select>
                                        </div>
                                    </Show>
                                </div>
                                <div class="flex flex-col gap-2">
                                    <select
                                        data-testid="form-travel-preset"
                                        aria-label="Travel time"
                                        class=SELECT
                                        prop:value=move || form.travel_preset.get()
                                        on:change=move |ev| form.travel_preset.set(event_target_value(&ev))
                                    >
                                        <option value="none">"No travel time"</option>
                                        <option value="0">"0 minutes"</option>
                                        <option value="15">"15 minutes"</option>
                                        <option value="30">"30 minutes"</option>
                                        <option value="45">"45 minutes"</option>
                                        <option value="60">"1 hour"</option>
                                        <option value="90">"1.5 hours"</option>
                                        <option value="custom">"Custom…"</option>
                                    </select>
                                    <Show when=move || form.travel_preset.get() == "custom">
                                        <div class="grid grid-cols-2 gap-2">
                                            <input
                                                data-testid="form-travel-value"
                                                class=FIELD
                                                type="number"
                                                min="0"
                                                placeholder="Value"
                                                prop:value=move || form.travel_value.get()
                                                on:input=move |ev| form.travel_value.set(event_target_value(&ev))
                                            />
                                            <select
                                                data-testid="form-travel-unit"
                                                aria-label="Travel time unit"
                                                class=SELECT
                                                prop:value=move || form.travel_unit.get()
                                                on:change=move |ev| form.travel_unit.set(event_target_value(&ev))
                                            >
                                                <option value="minutes">"minutes"</option>
                                                <option value="hours">"hours"</option>
                                                <option value="days">"days"</option>
                                            </select>
                                        </div>
                                    </Show>
                                </div>
                                <div class="flex items-center gap-2 pt-1">
                                    <PendingButton
                                        testid="form-submit"
                                        submit=true
                                        pending=saving
                                        label=Signal::derive(move || {
                                            if editing_event.get().is_some() { "Save" } else { "Add event" }.to_string()
                                        })
                                        busy="Saving…"
                                        class="flex-1 h-12 rounded-full bg-action text-canvas text-body \
                                               disabled:opacity-40 flex items-center justify-center gap-2"
                                    />
                                    <Show when=move || editing_event.get().is_some()>
                                        // `button-secondary-pill`: the ghost
                                        // half of the spec's two-CTA pairing.
                                        <button
                                            type="button"
                                            data-testid="form-cancel"
                                            class="h-12 px-5 rounded-full border border-action text-action text-body"
                                            on:click=cancel_edit
                                        >
                                            "Cancel"
                                        </button>
                                    </Show>
                                </div>
                            </form>
                        </div>
                        </Show>
                    </Show>
                </div>
            </div>
        </main>

        <Footer/>
    }
}
