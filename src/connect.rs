use crate::dav;
use crate::storage::{self, ConnectionConfig};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

const PRESETS: &[(&str, &str)] = &[
    ("Nextcloud", "https://cloud.example.com/remote.php/dav/"),
    ("Radicale", "https://caldav.example.com:5232/"),
    ("iCloud", "https://caldav.icloud.com/"),
    ("Fastmail", "https://caldav.fastmail.com/dav/"),
];

const INPUT_CLASSES: &str = "w-full bg-surface-inner border border-border-tech text-text-bright font-mono text-xs pl-10 pr-3 py-2.5 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary transition-all placeholder:text-text-muted/40";

#[component]
pub fn ConnectRoutePage() -> impl IntoView {
    let navigate = use_navigate();

    let server_url = RwSignal::new(String::new());
    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let password_visible = RwSignal::new(false);
    let connecting = RwSignal::new(false);
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

    let do_connect = move || {
        connecting.set(true);
        status.set(String::new());
        let config = ConnectionConfig {
            server_url: server_url.get_untracked(),
            username: username.get_untracked(),
            password: password.get_untracked(),
        };
        let navigate = navigate.clone();
        spawn_local(async move {
            match dav::list_calendars(&config).await {
                Ok(_) => {
                    storage::save(&config);
                    navigate("/calendar", Default::default());
                }
                Err(e) => {
                    status.set(format!("Connect failed: {e}"));
                    connecting.set(false);
                }
            }
        });
    };

    view! {
        <div class="pointer-events-none fixed inset-0 z-0 cad-grid-bg"></div>
        <div class="pointer-events-none fixed inset-0 z-0 cad-grid-major"></div>
        <div class="pointer-events-none fixed inset-0 z-0 bg-[radial-gradient(ellipse_at_50%_20%,rgba(43,232,201,0.07),transparent_70%)]"></div>

        <header class="relative z-10 w-full border-b border-border-tech bg-[#050508]/85 backdrop-blur-md px-4 lg:px-8 py-2.5 flex items-center gap-2 text-xs tracking-wider">
            <span class="inline-block w-2 h-2 bg-primary shadow-[0_0_8px_#2BE8C9]"></span>
            <span class="font-bold text-primary tracking-widest font-mono text-sm">"WEBCALDAV // V1"</span>
        </header>

        <main class="relative z-10 w-full flex-1 flex flex-col items-center px-4 py-8">
            <Show when=move || !status.get().is_empty()>
                <div class="w-full max-w-[520px] mb-4 flex items-start gap-2.5 p-3 border border-accent-orange bg-accent-orange/10 text-text-bright font-mono text-xs">
                    <span class="material-symbols-outlined text-[18px] text-accent-orange mt-0.5">"error"</span>
                    <span>{move || status.get()}</span>
                </div>
            </Show>

            <div class="w-full max-w-[520px] relative">
                <div class="absolute -top-2.5 -left-2.5 w-4 h-4 border-t-2 border-l-2 border-primary pointer-events-none"></div>
                <div class="absolute -top-2.5 -right-2.5 w-4 h-4 border-t-2 border-r-2 border-primary pointer-events-none"></div>
                <div class="absolute -bottom-2.5 -left-2.5 w-4 h-4 border-b-2 border-l-2 border-primary pointer-events-none"></div>
                <div class="absolute -bottom-2.5 -right-2.5 w-4 h-4 border-b-2 border-r-2 border-primary pointer-events-none"></div>

                <div class="relative bg-surface-panel border border-border-tech shadow-[0_0_35px_rgba(0,0,0,0.85)] p-6 sm:p-7 flex flex-col gap-5">
                    <div class="flex flex-col gap-2 border-b border-border-tech pb-4">
                        <span class="text-[10px] tracking-widest font-mono text-primary font-semibold bg-primary/10 border border-primary/30 px-1.5 py-0.5 w-fit">
                            "SYS_OK"
                        </span>
                        <h1 class="text-xl sm:text-2xl font-bold font-display text-text-bright tracking-tight uppercase">
                            "AUTHORIZE NODE"
                        </h1>
                        <p class="text-xs font-mono text-text-muted leading-relaxed">
                            "Establish an encrypted connection to your CalDAV calendar server."
                        </p>
                    </div>

                    <div class="flex flex-col gap-1.5">
                        <div class="flex items-center justify-between text-[10px] text-text-muted font-mono uppercase tracking-wider">
                            <span>"// QUICK-CONNECT PRESETS"</span>
                        </div>
                        <div class="grid grid-cols-4 gap-1.5 font-mono text-xs">
                            {PRESETS
                                .iter()
                                .map(|(name, url)| {
                                    let url = url.to_string();
                                    view! {
                                        <button
                                            type="button"
                                            class="px-2 py-1.5 bg-surface-inner border border-border-tech hover:border-primary hover:text-primary text-text-muted transition-colors text-center text-[11px] font-medium"
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
                        class="flex flex-col gap-4"
                        on:submit=move |ev: leptos::ev::SubmitEvent| {
                            ev.prevent_default();
                            do_connect();
                        }
                    >
                        <div class="flex flex-col gap-1">
                            <label class="text-primary font-semibold tracking-wider text-[11px] font-mono" for="server-url">
                                "01 // SERVER ENDPOINT URL"
                            </label>
                            <div class="relative flex items-center">
                                <div class="absolute left-3 flex items-center pointer-events-none text-primary">
                                    <span class="material-symbols-outlined text-[16px]">"dns"</span>
                                </div>
                                <input
                                    id="server-url"
                                    class=INPUT_CLASSES
                                    type="text"
                                    placeholder="https://notion-caldav.opendiy.vn/..."
                                    prop:value=move || server_url.get()
                                    on:input=move |ev| server_url.set(event_target_value(&ev))
                                    required
                                />
                            </div>
                        </div>

                        <div class="flex flex-col gap-1">
                            <label class="text-primary font-semibold tracking-wider text-[11px] font-mono" for="uid">
                                "02 // OPERATOR IDENTITY"
                            </label>
                            <div class="relative flex items-center">
                                <div class="absolute left-3 flex items-center pointer-events-none text-primary">
                                    <span class="material-symbols-outlined text-[16px]">"badge"</span>
                                </div>
                                <input
                                    id="uid"
                                    class=INPUT_CLASSES
                                    type="text"
                                    placeholder="you@example.com"
                                    prop:value=move || username.get()
                                    on:input=move |ev| username.set(event_target_value(&ev))
                                    required
                                />
                            </div>
                        </div>

                        <div class="flex flex-col gap-1">
                            <label class="text-primary font-semibold tracking-wider text-[11px] font-mono" for="secret-key">
                                "03 // ACCESS PASSWORD"
                            </label>
                            <div class="relative flex items-center">
                                <div class="absolute left-3 flex items-center pointer-events-none text-primary">
                                    <span class="material-symbols-outlined text-[16px]">"key"</span>
                                </div>
                                <input
                                    id="secret-key"
                                    class="w-full bg-surface-inner border border-border-tech text-text-bright font-mono text-xs pl-10 pr-10 py-2.5 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary transition-all placeholder:text-text-muted/40 tracking-widest"
                                    type=move || if password_visible.get() { "text" } else { "password" }
                                    placeholder="••••••••••••••••"
                                    prop:value=move || password.get()
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                    required
                                />
                                <button
                                    type="button"
                                    class="absolute right-3 text-text-muted hover:text-primary transition-colors flex items-center"
                                    on:click=move |_| password_visible.update(|v| *v = !*v)
                                >
                                    <span class="material-symbols-outlined text-[16px]">
                                        {move || if password_visible.get() { "visibility_off" } else { "visibility" }}
                                    </span>
                                </button>
                            </div>
                        </div>

                        <button
                            type="submit"
                            class="relative w-full mt-1 focus:outline-none"
                            disabled=move || connecting.get()
                        >
                            <div class="flex items-center justify-center gap-2 bg-accent-orange hover:bg-accent-orange-hover text-black font-mono font-bold text-sm tracking-widest py-3 px-6 transition-all shadow-[0_0_20px_rgba(255,85,0,0.35)]">
                                <span class="material-symbols-outlined text-[20px]">
                                    {move || if connecting.get() { "progress_activity" } else { "bolt" }}
                                </span>
                                <span>{move || if connecting.get() { "CONNECTING..." } else { "INITIALIZE HANDSHAKE" }}</span>
                            </div>
                        </button>
                    </form>
                </div>

                <div class="flex items-center justify-between px-1 mt-3 text-text-muted font-mono text-[11px]">
                    <div class="flex items-center gap-1.5">
                        <span class="material-symbols-outlined text-[14px] text-primary">"security"</span>
                        <span>"CREDENTIALS STORED LOCALLY IN THIS BROWSER ONLY"</span>
                    </div>
                </div>
            </div>
        </main>

        <footer class="relative z-10 w-full border-t border-border-tech bg-[#050508]/85 backdrop-blur-md px-4 lg:px-8 py-2.5 flex flex-col sm:flex-row items-center justify-between gap-2 text-text-muted font-mono text-xs">
            <div class="flex items-center gap-3">
                <span>"CALENDAR DISCOVERY PROTOCOL"</span>
                <span class="text-border-tech">"/"</span>
                <span class="flex items-center gap-1">
                    <span class="w-1.5 h-1.5 bg-primary"></span>
                    <span>"SYNC ENGINE: IDLE"</span>
                </span>
            </div>
            <div class="flex items-center gap-2 text-[11px]">
                <span class="text-primary">"SECURE PROTOCOL"</span>
                <span class="text-border-tech">"//"</span>
                <span>"WEBCALDAV V1.0"</span>
            </div>
        </footer>
    }
}
