use crate::dav::{self, CalendarSummary, EventSummary, NewEvent};
use crate::layout::{Footer, TopBar};
#[cfg_attr(not(feature = "hydrate"), allow(unused_imports))]
use crate::storage::{self, CalendarPrefs, ConnectionConfig};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use std::collections::{HashMap, HashSet};

const REMINDER_PRESETS: &[(&str, i64)] = &[("5", 5), ("15", 15), ("30", 30), ("60", 60), ("1440", 1440), ("2880", 2880), ("10080", 10080)];
const TRAVEL_PRESETS: &[(&str, i64)] = &[("0", 0), ("15", 15), ("30", 30), ("45", 45), ("60", 60), ("90", 90)];
const COLOR_POOL: &[&str] = &["#2BE8C9", "#FF5500", "#00F5FF", "#A78BFA", "#F472B6", "#34D399", "#F87171", "#60A5FA", "#FBBF24", "#FB923C"];

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

fn slot_label(count: usize) -> String {
    match count {
        0 => "FREE".to_string(),
        1 => "1 SLOT".to_string(),
        n => format!("{n} SLOTS"),
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
    let loading = RwSignal::new(true);
    let status = RwSignal::new(String::new());
    let latency_ms = RwSignal::new(Option::<i64>::None);
    let last_synced = RwSignal::new(Option::<String>::None);

    let today = chrono::Local::now().date_naive();
    let current_month = RwSignal::new(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap());
    let selected_day = RwSignal::new(Option::<NaiveDate>::None);

    let editing_event = RwSignal::new(Option::<EventSummary>::None);
    let form = EventForm::new();

    let calendar_color = move |calendar_url: &str| -> String {
        calendar_colors.get().get(calendar_url).cloned().unwrap_or_else(|| "#2BE8C9".to_string())
    };

    let refresh_events = move || {
        let Some(cfg) = config.get_untracked() else { return };
        let urls: Vec<String> = calendars.get_untracked().iter().map(|c| c.url.clone()).collect();
        if urls.is_empty() {
            return;
        }
        loading.set(true);
        spawn_local(async move {
            match dav::list_events_for_calendars(&cfg, &urls).await {
                Ok(fetch) => {
                    events.set(fetch.events);
                    latency_ms.set(Some(fetch.latency_ms));
                    last_synced.set(Some(chrono::Local::now().format("%H:%M:%S").to_string()));
                    status.set(String::new());
                }
                Err(e) => status.set(format!("Failed to load events: {e}")),
            }
            loading.set(false);
        });
    };

    #[cfg(feature = "hydrate")]
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let Some(cfg) = storage::load() else {
                navigate("/", Default::default());
                return;
            };
            username.set(cfg.username.clone());
            config.set(Some(cfg.clone()));
            spawn_local(async move {
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
            });
        });
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

    let select_day = move |day: NaiveDate| {
        selected_day.set(Some(day));
        editing_event.set(None);
        form.clear();
        form.calendar_url.set(first_visible_calendar());
        form.set_default_day(day);
    };

    let submit_event = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(cfg) = config.get_untracked() else { return };
        let target_calendar = form.calendar_url.get_untracked();
        let new_event = form.to_new_event();
        let editing = editing_event.get_untracked();
        spawn_local(async move {
            let result = match editing {
                Some(existing) => dav::update_event(&cfg, &existing, &target_calendar, new_event).await,
                None => dav::create_event(&cfg, &target_calendar, new_event).await,
            };
            match result {
                Ok(()) => {
                    editing_event.set(None);
                    form.clear();
                    refresh_events();
                }
                Err(e) => status.set(format!("Save failed: {e}")),
            }
        });
    };

    let start_edit = move |event: EventSummary| {
        form.fill_from(&event);
        editing_event.set(Some(event));
    };

    let cancel_edit = move |_| {
        editing_event.set(None);
        form.clear();
    };

    let remove_event = move |event: EventSummary| {
        let Some(cfg) = config.get_untracked() else { return };
        spawn_local(async move {
            match dav::delete_event(&cfg, &event).await {
                Ok(()) => refresh_events(),
                Err(e) => status.set(format!("Delete failed: {e}")),
            }
        });
    };

    let events_on = move |day: NaiveDate| -> Vec<EventSummary> {
        let prefix = day.format("%Y-%m-%d").to_string();
        let hidden = hidden_calendars.get();
        events.get().into_iter().filter(|e| e.start.starts_with(&prefix) && !hidden.contains(&e.calendar_url)).collect()
    };

    view! {
        <div class="fixed inset-0 pointer-events-none z-0">
            <div class="absolute inset-0 bg-blueprint-grid opacity-75"></div>
            <div class="absolute -top-40 left-1/2 -translate-x-1/2 w-[760px] h-[480px] bg-gradient-to-b from-[#2BE8C9]/10 via-[#00F5FF]/5 to-transparent blur-[130px] rounded-full"></div>
            <div class="absolute bottom-0 right-1/4 w-[420px] h-[300px] bg-[#FF5500]/[0.04] blur-[110px] rounded-full"></div>
        </div>

        <TopBar username=username.get_untracked()/>

        <main class="w-full flex-1 relative z-10">
            <div class="flex flex-col w-full px-4 sm:px-6 py-6 max-w-[1280px] mx-auto">
                <Show when=move || !status.get().is_empty()>
                    <div class="flex items-start gap-2.5 p-3 mb-4 rounded border border-[#FF5500] bg-[#FF5500]/10 text-[#e6edf3] font-mono text-xs">
                        <span class="material-symbols-outlined text-[18px] text-[#FF5500] mt-0.5">"error"</span>
                        <span>{move || status.get()}</span>
                    </div>
                </Show>

                <div class="flex flex-wrap items-center justify-between gap-4 mb-5">
                    <div class="flex flex-wrap items-center gap-3">
                        <span class="px-3 py-1 rounded bg-[#0D1117] border border-[#2BE8C9]/30 text-[#2BE8C9] font-mono text-xs tracking-wider uppercase flex items-center gap-2 shadow-[0_0_12px_rgba(43,232,201,0.15)]">
                            <span class="material-symbols-outlined text-[14px]">"terminal"</span>
                            "DAV ENGINE: RFC-4791"
                        </span>
                        <span class="text-[#8B949E] font-mono text-xs tracking-tight">
                            {move || calendars.get().len()}" CALENDARS"
                        </span>
                    </div>
                    <div class="flex items-center gap-4 text-[#8B949E] font-mono text-xs">
                        <Show when=move || last_synced.get().is_some()>
                            <span class="flex items-center gap-1.5 text-[#2BE8C9]">
                                <span class="material-symbols-outlined text-[15px]">"cloud_done"</span>
                                "SYNCED "{move || last_synced.get().unwrap_or_default()}
                            </span>
                        </Show>
                        <Show when=move || latency_ms.get().is_some()>
                            <span class="text-white/20">"|"</span>
                            <span class="text-[#FF5500] font-semibold">
                                "LATENCY: "{move || latency_ms.get().unwrap_or_default()}"ms"
                            </span>
                        </Show>
                    </div>
                </div>

                <div class="bg-[#0D1117]/95 border border-[#2BE8C9]/30 rounded p-4 lg:p-5 shadow-2xl flex flex-col xl:flex-row items-stretch xl:items-center justify-between gap-4 mb-6 backdrop-blur-xl relative cyan-glow-subtle">
                    <div class="absolute -top-[3px] -left-[3px] w-2.5 h-2.5 border-t-2 border-l-2 border-[#2BE8C9] pointer-events-none"></div>
                    <div class="absolute -top-[3px] -right-[3px] w-2.5 h-2.5 border-t-2 border-r-2 border-[#2BE8C9] pointer-events-none"></div>
                    <div class="absolute -bottom-[3px] -left-[3px] w-2.5 h-2.5 border-b-2 border-l-2 border-[#FF5500] pointer-events-none"></div>
                    <div class="absolute -bottom-[3px] -right-[3px] w-2.5 h-2.5 border-b-2 border-r-2 border-[#FF5500] pointer-events-none"></div>

                    <div class="flex items-center gap-3">
                        <div class="flex items-center bg-[#050508] border border-[#2BE8C9]/30 rounded p-1 shadow-inner">
                            <button
                                class="w-8 h-8 rounded flex items-center justify-center text-[#8B949E] hover:text-[#2BE8C9] hover:bg-[#131722] transition-all"
                                on:click=move |_| current_month.update(|d| *d = shift_month(*d, -1))
                            >
                                <span class="material-symbols-outlined text-[18px]">"chevron_left"</span>
                            </button>
                            <div class="px-3 flex items-center gap-2">
                                <span class="material-symbols-outlined text-[#2BE8C9] text-[17px]">"calendar_month"</span>
                                <span class="font-mono font-bold tracking-widest text-[#e6edf3] uppercase text-xs">
                                    {move || current_month.get().format("%B %Y").to_string()}
                                </span>
                            </div>
                            <button
                                class="w-8 h-8 rounded flex items-center justify-center text-[#8B949E] hover:text-[#2BE8C9] hover:bg-[#131722] transition-all"
                                on:click=move |_| current_month.update(|d| *d = shift_month(*d, 1))
                            >
                                <span class="material-symbols-outlined text-[18px]">"chevron_right"</span>
                            </button>
                        </div>
                        <button
                            class="hidden sm:inline-flex px-3 py-1.5 rounded bg-[#131722] border border-[#2BE8C9]/30 hover:border-[#2BE8C9] text-[#2BE8C9] font-mono text-xs font-semibold tracking-wider transition-all"
                            on:click=move |_| current_month.set(NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap())
                        >
                            "[ TODAY ]"
                        </button>
                        <Show when=move || loading.get()>
                            <span class="text-[#8B949E] font-mono text-xs">"SYNCING..."</span>
                        </Show>
                    </div>

                    <div class="flex items-center self-start xl:self-center bg-[#050508] border border-[#2BE8C9]/20 p-1 rounded font-mono text-xs">
                        <button class="px-3 py-1.5 rounded bg-[#131722] text-[#2BE8C9] font-bold border border-[#2BE8C9]/40 shadow-[0_0_8px_rgba(43,232,201,0.25)] transition-all">
                            "[ MONTH ]"
                        </button>
                        <button
                            class="px-3 py-1.5 rounded text-[#8B949E] opacity-40 cursor-not-allowed"
                            disabled
                            title="Week view not implemented yet"
                        >
                            "[ WEEK ]"
                        </button>
                        <button
                            class="px-3 py-1.5 rounded text-[#8B949E] opacity-40 cursor-not-allowed"
                            disabled
                            title="Timeline view not implemented yet"
                        >
                            "[ TIMELINE ]"
                        </button>
                    </div>

                    <button
                        class="ml-auto xl:ml-2 flex items-center gap-2 px-4 py-2 rounded bg-[#FF5500] hover:bg-[#FF6B00] text-[#050508] font-mono font-bold text-xs uppercase tracking-widest orange-glow-btn transition-all active:translate-y-0.5 cursor-pointer"
                        on:click=move |_| select_day(selected_day.get_untracked().unwrap_or(today))
                    >
                        <span class="material-symbols-outlined text-[17px]">"bolt"</span>
                        "+ ADD EVENT"
                    </button>
                </div>

                <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
                    <div class="lg:col-span-8 xl:col-span-9 flex flex-col gap-4">
                        <div class="grid grid-cols-7 gap-2 px-2 text-center font-mono text-xs">
                            {["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"]
                                .iter()
                                .enumerate()
                                .map(|(i, d)| {
                                    let is_weekend = i >= 5;
                                    view! {
                                        <div
                                            class="tracking-widest py-1"
                                            class=("text-[#FF5500]", is_weekend)
                                            class=("font-semibold", is_weekend)
                                            class=("text-[#8B949E]", !is_weekend)
                                        >
                                            {*d}
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>

                        <div class="grid grid-cols-7 gap-2 bg-[#0D1117] border border-[#2BE8C9]/30 p-2.5 rounded shadow-2xl">
                            {move || {
                                grid_cells(current_month.get())
                                    .into_iter()
                                    .map(|(day, in_month)| {
                                        let is_today = day == today;
                                        let is_weekend = matches!(day.weekday(), Weekday::Sat | Weekday::Sun);
                                        let is_selected = move || selected_day.get() == Some(day);
                                        let day_events = events_on(day);
                                        let overflow = day_events.len().saturating_sub(3);
                                        let count_label = slot_label(day_events.len());
                                        view! {
                                            <div
                                                class="min-h-[110px] xl:min-h-[125px] p-2 rounded flex flex-col justify-between cursor-pointer transition-all font-mono"
                                                class=("opacity-35", move || !in_month)
                                                class=("bg-[#131722]", move || is_today)
                                                class=("bg-[#050508]", move || !is_today)
                                                class=("border-2", move || is_today || is_selected())
                                                class=("border", move || !(is_today || is_selected()))
                                                class=("border-[#2BE8C9]", move || is_today || is_selected())
                                                class=("shadow-[0_0_16px_rgba(43,232,201,0.3)]", move || is_today || is_selected())
                                                class=("border-[#2BE8C9]/20", move || !(is_today || is_selected()))
                                                class=("hover:border-[#2BE8C9]/50", move || !(is_today || is_selected()))
                                                on:click=move |_| select_day(day)
                                            >
                                                <div class="flex items-center justify-between">
                                                    <span
                                                        class="text-xs"
                                                        class=("text-[#00F5FF]", move || is_today)
                                                        class=("font-bold", move || is_today)
                                                        class=("flex", move || is_today)
                                                        class=("items-center", move || is_today)
                                                        class=("gap-1.5", move || is_today)
                                                        class=("text-[#FF6B00]", move || !is_today && is_weekend)
                                                        class=("text-[#e6edf3]", move || !is_today && !is_weekend)
                                                    >
                                                        {day.day()}
                                                    </span>
                                                    <Show when=move || is_today>
                                                        <span class="text-[9px] px-1 py-0.5 rounded bg-[#2BE8C9] text-[#050508] uppercase font-bold tracking-tight">"NOW"</span>
                                                    </Show>
                                                </div>
                                                <div class="flex flex-col gap-1 my-1">
                                                    {day_events
                                                        .iter()
                                                        .take(3)
                                                        .map(|e| {
                                                            let dot = calendar_color(&e.calendar_url);
                                                            view! {
                                                                <div class="px-1.5 py-0.5 rounded bg-[#131722] text-[#e6edf3] text-[11px] truncate flex items-center gap-1">
                                                                    <span class="w-1.5 h-1.5 rounded-full shrink-0" style:background-color=dot></span>
                                                                    <span>{e.summary.clone()}</span>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()}
                                                    <Show when=move || { overflow > 0 }>
                                                        <div class="text-[9px] text-[#8B949E]">{format!("+{overflow} more")}</div>
                                                    </Show>
                                                </div>
                                                <div class="text-[9px] text-[#8B949E] text-right">{count_label}</div>
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            }}
                        </div>
                    </div>

                    <div class="lg:col-span-4 xl:col-span-3 flex flex-col gap-5">
                        <div class="bg-[#0D1117] border border-[#2BE8C9]/30 rounded p-4 shadow-2xl flex flex-col gap-3">
                            <span class="font-mono text-[10px] text-[#2BE8C9] tracking-widest uppercase font-semibold">"CALENDARS"</span>
                            <div class="flex flex-col gap-2 font-mono text-xs">
                                <For
                                    each=move || calendars.get()
                                    key=|c| c.url.clone()
                                    children=move |cal: CalendarSummary| {
                                        let url_for_toggle = cal.url.clone();
                                        let url_for_visible = cal.url.clone();
                                        let url_for_color_get = cal.url.clone();
                                        let url_for_color_set = cal.url.clone();
                                        view! {
                                            <label class="flex items-center gap-2 cursor-pointer group">
                                                <input
                                                    type="checkbox"
                                                    class="accent-[#2BE8C9]"
                                                    prop:checked=move || !hidden_calendars.get().contains(&url_for_visible)
                                                    on:change=move |_| toggle_calendar_visible(url_for_toggle.clone())
                                                />
                                                <input
                                                    type="color"
                                                    class="w-5 h-5 rounded border-none bg-transparent cursor-pointer"
                                                    prop:value=move || calendar_color(&url_for_color_get)
                                                    on:input=move |ev| set_calendar_color(url_for_color_set.clone(), event_target_value(&ev))
                                                />
                                                <span class="text-[#e6edf3] truncate group-hover:text-[#2BE8C9] transition-colors">
                                                    {cal.display_name.clone()}
                                                </span>
                                            </label>
                                        }
                                    }
                                />
                            </div>
                        </div>

                        <div class="bg-[#0D1117] border border-[#2BE8C9]/35 rounded p-5 shadow-2xl flex flex-col gap-4 relative cyan-glow-subtle">
                            <div class="absolute -top-1 -left-1 w-3 h-3 border-t-2 border-l-2 border-[#2BE8C9]"></div>
                            <div class="absolute -top-1 -right-1 w-3 h-3 border-t-2 border-r-2 border-[#2BE8C9]"></div>
                            <div class="absolute -bottom-1 -left-1 w-3 h-3 border-b-2 border-l-2 border-[#FF5500]"></div>
                            <div class="absolute -bottom-1 -right-1 w-3 h-3 border-b-2 border-r-2 border-[#FF5500]"></div>

                            <Show
                                when=move || selected_day.get().is_some()
                                fallback=|| view! {
                                    <p class="text-[#8B949E] font-mono text-xs">"Click a day on the grid to view or add events."</p>
                                }
                            >
                                <div class="flex items-center justify-between pb-3 bg-[#050508] border-b border-[#2BE8C9]/20 -mx-5 -mt-5 p-4 rounded-t">
                                    <div class="flex flex-col">
                                        <span class="font-mono text-[10px] text-[#2BE8C9] tracking-widest uppercase font-semibold">"DAY INSPECTOR"</span>
                                        <span class="font-mono text-xs text-[#e6edf3] font-bold tracking-wider">
                                            {move || selected_day.get().map(|d| d.format("%B %-d, %Y").to_string()).unwrap_or_default()}
                                        </span>
                                    </div>
                                    <span class="px-2 py-0.5 rounded bg-[#2BE8C9]/15 border border-[#2BE8C9]/40 text-[#2BE8C9] font-mono text-[11px] font-bold">
                                        {move || selected_day.get().map(events_on).unwrap_or_default().len()}" EVENTS"
                                    </span>
                                </div>

                                <div class="flex flex-col gap-3 font-mono">
                                    <For
                                        each=move || selected_day.get().map(events_on).unwrap_or_default()
                                        key=|e| e.uid.clone()
                                        children=move |event: EventSummary| {
                                            let event_for_edit = event.clone();
                                            let event_for_delete = event.clone();
                                            let dot = calendar_color(&event.calendar_url);
                                            view! {
                                                <div class="p-3 rounded bg-[#050508] border border-[#2BE8C9]/20 hover:border-[#2BE8C9]/40 transition-all flex flex-col gap-1.5 group">
                                                    <div class="flex items-center justify-between">
                                                        <span class="px-2 py-0.5 rounded bg-[#131722] text-[#e6edf3] text-[10px] font-bold">
                                                            {event.start.clone()}" - "{event.end.clone()}
                                                        </span>
                                                        <span class="flex items-center gap-1">
                                                            <button
                                                                class="w-6 h-6 flex items-center justify-center text-[#8B949E] hover:text-[#2BE8C9] transition-colors"
                                                                on:click=move |_| start_edit(event_for_edit.clone())
                                                            >
                                                                <span class="material-symbols-outlined text-[14px]">"edit"</span>
                                                            </button>
                                                            <button
                                                                class="w-6 h-6 flex items-center justify-center text-[#8B949E] hover:text-[#FF5500] transition-colors"
                                                                on:click=move |_| remove_event(event_for_delete.clone())
                                                            >
                                                                <span class="material-symbols-outlined text-[14px]">"delete"</span>
                                                            </button>
                                                        </span>
                                                    </div>
                                                    <div class="text-xs text-[#e6edf3] font-bold group-hover:text-[#2BE8C9] transition-colors flex items-center gap-1.5">
                                                        <span class="w-1.5 h-1.5 rounded-full shrink-0" style:background-color=dot></span>
                                                        <span>{event.summary.clone()}</span>
                                                    </div>
                                                    <Show when={
                                                        let d = event.description.clone();
                                                        move || !d.is_empty()
                                                    }>
                                                        <div class="text-[#8B949E] text-[11px]">{event.description.clone()}</div>
                                                    </Show>
                                                </div>
                                            }
                                        }
                                    />
                                </div>

                                <div class="mt-2 pt-4 bg-[#050508] border-t border-[#2BE8C9]/20 -mx-5 -mb-5 p-4 rounded-b flex flex-col gap-3 font-mono">
                                    <div class="flex items-center justify-between">
                                        <span class="text-xs text-[#2BE8C9] font-bold tracking-wide flex items-center gap-1.5 uppercase">
                                            <span class="material-symbols-outlined text-[16px]">"bolt"</span>
                                            {move || if editing_event.get().is_some() { "EDIT EVENT" } else { "+ QUICK SCHEDULE" }}
                                        </span>
                                        <button
                                            type="button"
                                            class="flex items-center gap-1 px-2 py-1 rounded border border-[#2BE8C9]/25 text-[#8B949E] hover:text-[#2BE8C9] hover:border-[#2BE8C9]/60 font-mono text-[10px] uppercase tracking-wider transition-colors cursor-pointer"
                                            title="Fill with random test data"
                                            on:click=move |_| random_fill(form)
                                        >
                                            <span class="material-symbols-outlined text-[13px]">"casino"</span>
                                            "RANDOM"
                                        </button>
                                    </div>
                                    <form on:submit=submit_event class="flex flex-col gap-2">
                                        <select
                                            class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                            prop:value=move || form.calendar_url.get()
                                            on:change=move |ev| form.calendar_url.set(event_target_value(&ev))
                                        >
                                            {move || calendars.get().into_iter().map(|c| view! {
                                                <option value=c.url.clone()>{c.display_name.clone()}</option>
                                            }).collect_view()}
                                        </select>
                                        <input
                                            class="w-full bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] placeholder:text-[#8B949E] text-xs px-3 py-2 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                            type="text"
                                            placeholder="Event title"
                                            prop:value=move || form.summary.get()
                                            on:input=move |ev| form.summary.set(event_target_value(&ev))
                                            required
                                        />
                                        <textarea
                                            class="w-full bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] placeholder:text-[#8B949E] text-xs px-3 py-2 rounded focus:outline-none focus:border-[#2BE8C9] transition-all resize-none"
                                            rows="2"
                                            placeholder="Description"
                                            prop:value=move || form.description.get()
                                            on:input=move |ev| form.description.set(event_target_value(&ev))
                                        ></textarea>
                                        <label class="flex items-center gap-2 cursor-pointer font-mono text-xs text-[#e6edf3]">
                                            <input
                                                type="checkbox"
                                                class="accent-[#2BE8C9]"
                                                prop:checked=move || form.all_day.get()
                                                on:change=move |_| form.toggle_all_day()
                                            />
                                            <span>"All day"</span>
                                        </label>
                                        <div class="grid grid-cols-2 gap-2">
                                            <input
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#2BE8C9] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                type=move || if form.all_day.get() { "date" } else { "datetime-local" }
                                                prop:value=move || form.start.get()
                                                on:input=move |ev| form.start.set(event_target_value(&ev))
                                                required
                                            />
                                            <input
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#2BE8C9] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                type=move || if form.all_day.get() { "date" } else { "datetime-local" }
                                                prop:value=move || form.end.get()
                                                on:input=move |ev| form.end.set(event_target_value(&ev))
                                                required
                                            />
                                        </div>
                                        <input
                                            class="w-full bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] placeholder:text-[#8B949E] text-xs px-3 py-2 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                            type="text"
                                            placeholder="Location"
                                            prop:value=move || form.location.get()
                                            on:input=move |ev| form.location.set(event_target_value(&ev))
                                        />
                                        <div class="grid grid-cols-2 gap-2">
                                            <select
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                prop:value=move || form.priority.get()
                                                on:change=move |ev| form.priority.set(event_target_value(&ev))
                                            >
                                                <option value="">"Priority: Not set"</option>
                                                <option value="1">"High"</option>
                                                <option value="5">"Medium"</option>
                                                <option value="9">"Low"</option>
                                            </select>
                                            <select
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                prop:value=move || form.busy.get()
                                                on:change=move |ev| form.busy.set(event_target_value(&ev))
                                            >
                                                <option value="">"Status: Not set"</option>
                                                <option value="busy">"Busy"</option>
                                                <option value="free">"Free"</option>
                                            </select>
                                        </div>
                                        <div class="flex flex-col gap-1.5">
                                            <select
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                prop:value=move || form.reminder_preset.get()
                                                on:change=move |ev| form.reminder_preset.set(event_target_value(&ev))
                                            >
                                                <option value="none">"Reminder: None"</option>
                                                <option value="5">"5 minutes before"</option>
                                                <option value="15">"15 minutes before"</option>
                                                <option value="30">"30 minutes before"</option>
                                                <option value="60">"1 hour before"</option>
                                                <option value="1440">"1 day before"</option>
                                                <option value="2880">"2 days before"</option>
                                                <option value="10080">"1 week before"</option>
                                                <option value="custom">"Custom..."</option>
                                            </select>
                                            <Show when=move || form.reminder_preset.get() == "custom">
                                                <div class="grid grid-cols-2 gap-2">
                                                    <input
                                                        class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                        type="number"
                                                        min="0"
                                                        placeholder="Value"
                                                        prop:value=move || form.reminder_value.get()
                                                        on:input=move |ev| form.reminder_value.set(event_target_value(&ev))
                                                    />
                                                    <select
                                                        class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
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
                                        <div class="flex flex-col gap-1.5">
                                            <select
                                                class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                prop:value=move || form.travel_preset.get()
                                                on:change=move |ev| form.travel_preset.set(event_target_value(&ev))
                                            >
                                                <option value="none">"Travel time: None"</option>
                                                <option value="0">"0 minutes"</option>
                                                <option value="15">"15 minutes"</option>
                                                <option value="30">"30 minutes"</option>
                                                <option value="45">"45 minutes"</option>
                                                <option value="60">"1 hour"</option>
                                                <option value="90">"1.5 hours"</option>
                                                <option value="custom">"Custom..."</option>
                                            </select>
                                            <Show when=move || form.travel_preset.get() == "custom">
                                                <div class="grid grid-cols-2 gap-2">
                                                    <input
                                                        class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
                                                        type="number"
                                                        min="0"
                                                        placeholder="Value"
                                                        prop:value=move || form.travel_value.get()
                                                        on:input=move |ev| form.travel_value.set(event_target_value(&ev))
                                                    />
                                                    <select
                                                        class="bg-[#131722] border border-[#2BE8C9]/30 text-[#e6edf3] text-xs px-3 py-1.5 rounded focus:outline-none focus:border-[#2BE8C9] transition-all"
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
                                        <div class="flex items-center gap-2">
                                            <button
                                                type="submit"
                                                class="flex-1 py-2.5 rounded bg-[#131722] border border-[#2BE8C9]/50 hover:bg-[#2BE8C9] hover:text-[#050508] text-[#2BE8C9] font-mono text-xs font-bold tracking-wider transition-all shadow-sm active:translate-y-0.5 flex items-center justify-center gap-1.5 cursor-pointer"
                                            >
                                                <span class="material-symbols-outlined text-[16px]">"add_circle"</span>
                                                {move || if editing_event.get().is_some() { "SAVE" } else { "COMMIT TO CALDAV" }}
                                            </button>
                                            <Show when=move || editing_event.get().is_some()>
                                                <button
                                                    type="button"
                                                    class="px-3 py-2.5 rounded border border-[#2BE8C9]/20 text-[#8B949E] hover:text-[#e6edf3] font-mono text-xs uppercase tracking-wider transition-colors"
                                                    on:click=cancel_edit
                                                >
                                                    "CANCEL"
                                                </button>
                                            </Show>
                                        </div>
                                    </form>
                                </div>
                            </Show>
                        </div>
                    </div>
                </div>
            </div>
        </main>

        <Footer/>
    }
}
