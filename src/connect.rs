use crate::dav;
use crate::storage::{self, ConnectionConfig};
use crate::ui::PendingButton;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

const PRESETS: &[(&str, &str)] = &[
    ("Nextcloud", "https://cloud.example.com/remote.php/dav/"),
    ("Radicale", "https://caldav.example.com:5232/"),
    ("iCloud", "https://caldav.icloud.com/"),
    ("Fastmail", "https://caldav.fastmail.com/dav/"),
];

/// `search-input` from DESIGN.md: pill, hairline, 17px, 44px tall.
/// `action-focus` is the one sanctioned second blue, used only for focus.
const INPUT_CLASSES: &str = "w-full h-11 bg-canvas border border-black/[0.08] rounded-full \
    text-body text-ink placeholder:text-ink-48 pl-11 pr-4 \
    focus:outline-none focus:border-action-focus focus:ring-1 focus:ring-action-focus \
    transition-colors";

#[component]
pub fn ConnectRoutePage() -> impl IntoView {
    let navigate = use_navigate();

    let server_url = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_visible = RwSignal::new(false);
    let status = RwSignal::new(String::new());

    #[cfg(feature = "hydrate")]
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            if storage::load().is_some() {
                navigate("/calendar", Default::default());
            }
        });
    }

    // An Action rather than spawn_local plus a hand-managed bool. `.pending()`
    // is derived from an in-flight counter, so it cannot be left stuck true by
    // an early return -- which is exactly how this kind of flag goes wrong.
    // `new_local` because the CalDAV futures are not Send.
    let connect = Action::new_local({
        let navigate = navigate.clone();
        move |_: &()| {
            let config = ConnectionConfig {
                server_url: server_url.get_untracked(),
                username: username.get_untracked(),
                password: password.get_untracked(),
            };
            let navigate = navigate.clone();
            async move {
                status.set(String::new());
                match dav::list_calendars(&config).await {
                    Ok(_) => {
                        storage::save(&config);
                        navigate("/calendar", Default::default());
                    }
                    Err(e) => status.set(format!("Connect failed: {e}")),
                }
            }
        }
    });
    let connecting = connect.pending();

    view! {
        // No decorative background layers: DESIGN.md forbids gradients.
        <main class="w-full flex-1 flex flex-col px-4 py-8">
            <div class="max-w-app mx-auto w-full flex flex-col gap-6">

                <div class="flex flex-col gap-2">
                    <h1 class="text-title font-semibold text-ink">"Connect a calendar"</h1>
                    <p class="text-body text-ink-80">
                        "Sign in to your CalDAV server to see your calendars here."
                    </p>
                </div>

                <Show when=move || !status.get().is_empty()>
                    // One accent only, so an error reads through wording and
                    // placement rather than colour.
                    <div
                        role="alert"
                        data-testid="connect-error"
                        class="flex items-start gap-2.5 p-4 rounded-lg bg-canvas border border-hairline"
                    >
                        <span class="material-symbols-outlined text-[20px] text-ink-48 shrink-0">
                            "error_outline"
                        </span>
                        <span class="text-caption text-ink-80">{move || status.get()}</span>
                    </div>
                </Show>

                // `store-utility-card`. No shadow, per DESIGN.md.
                <div class="bg-canvas border border-hairline rounded-lg p-6 flex flex-col gap-6">

                    <div class="flex flex-col gap-2.5">
                        <span class="text-caption font-semibold text-ink">"Quick connect"</span>
                        <div class="flex flex-wrap gap-2">
                            {PRESETS
                                .iter()
                                .map(|(name, url)| {
                                    let url = url.to_string();
                                    view! {
                                        // `configurator-option-chip`.
                                        <button
                                            type="button"
                                            data-testid="preset"
                                            data-preset=*name
                                            class="px-4 py-2 rounded-full border border-hairline \
                                                   text-caption text-ink bg-canvas \
                                                   hover:border-action hover:text-action"
                                            on:click=move |_| server_url.set(url.clone())
                                        >
                                            {*name}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>

                    <form
                        data-testid="connect-form"
                        class="flex flex-col gap-5"
                        on:submit=move |ev: leptos::ev::SubmitEvent| {
                            ev.prevent_default();
                            connect.dispatch(());
                        }
                    >
                        <div class="flex flex-col gap-2">
                            <label class="text-caption font-semibold text-ink" for="server-url">
                                "Server address"
                            </label>
                            <div class="relative flex items-center">
                                <span class="material-symbols-outlined absolute left-4 text-[18px] text-ink-48 pointer-events-none">
                                    "dns"
                                </span>
                                <input
                                    id="server-url"
                                    data-testid="server-url"
                                    class=INPUT_CLASSES
                                    type="url"
                                    inputmode="url"
                                    autocomplete="url"
                                    placeholder="https://example.com/dav/"
                                    prop:value=move || server_url.get()
                                    on:input=move |ev| server_url.set(event_target_value(&ev))
                                    required
                                />
                            </div>
                        </div>

                        <div class="flex flex-col gap-2">
                            <label class="text-caption font-semibold text-ink" for="uid">
                                "Username"
                            </label>
                            <div class="relative flex items-center">
                                <span class="material-symbols-outlined absolute left-4 text-[18px] text-ink-48 pointer-events-none">
                                    "person"
                                </span>
                                <input
                                    id="uid"
                                    data-testid="username"
                                    class=INPUT_CLASSES
                                    type="text"
                                    inputmode="email"
                                    autocomplete="username"
                                    placeholder="you@example.com"
                                    prop:value=move || username.get()
                                    on:input=move |ev| username.set(event_target_value(&ev))
                                    required
                                />
                            </div>
                        </div>

                        <div class="flex flex-col gap-2">
                            <label class="text-caption font-semibold text-ink" for="secret-key">
                                "Password"
                            </label>
                            <div class="relative flex items-center">
                                <span class="material-symbols-outlined absolute left-4 text-[18px] text-ink-48 pointer-events-none">
                                    "lock"
                                </span>
                                <input
                                    id="secret-key"
                                    data-testid="password"
                                    class=INPUT_CLASSES
                                    type=move || if password_visible.get() { "text" } else { "password" }
                                    autocomplete="current-password"
                                    placeholder="App password"
                                    prop:value=move || password.get()
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                    required
                                />
                                <button
                                    type="button"
                                    // 44px: the platform's minimum touch target.
                                    class="absolute right-1 w-11 h-11 flex items-center justify-center \
                                           text-ink-48 rounded-full"
                                    data-testid="password-toggle"
                                    aria-label="Show or hide password"
                                    on:click=move |_| password_visible.update(|v| *v = !*v)
                                >
                                    <span class="material-symbols-outlined text-[18px]">
                                        {move || if password_visible.get() { "visibility_off" } else { "visibility" }}
                                    </span>
                                </button>
                            </div>
                        </div>

                        // `button-primary`. Press-scale is global in lib.rs.
                        <PendingButton
                            testid="connect-submit"
                            submit=true
                            pending=connecting
                            label="Connect"
                            busy="Connecting…"
                            class="w-full h-12 rounded-full bg-action text-canvas text-body \
                                   disabled:opacity-40 flex items-center justify-center gap-2"
                        />
                    </form>
                </div>

                <p class="text-fine text-ink-48 px-1">
                    "Your credentials stay in this browser. They are never sent anywhere else."
                </p>
            </div>
        </main>
    }
}
