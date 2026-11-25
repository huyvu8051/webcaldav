use crate::storage;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

/// Sticky top bar — `sub-nav-frosted` from DESIGN.md.
///
/// Background spans the viewport, contents align to the same `max-w-app`
/// column as the page. That pairing is the entire desktop story: one
/// mobile-shaped layout with the background expanding around it.
#[component]
pub fn TopBar(#[prop(into)] username: Signal<String>) -> impl IntoView {
    let navigate = use_navigate();
    let disconnect = move |_| {
        storage::clear();
        navigate("/", Default::default());
    };
    view! {
        <header
            data-testid="topbar"
            class="sticky top-0 z-50 w-full bg-parchment/80 backdrop-blur-xl border-b border-hairline"
        >
            <div class="max-w-app mx-auto h-[52px] px-4 flex items-center justify-between gap-3">
                <div class="min-w-0">
                    <h1 class="text-tagline font-semibold text-ink truncate">"Calendar"</h1>
                </div>
                <div class="flex items-center gap-3 shrink-0">
                    <span data-testid="topbar-username" class="text-caption text-ink-48 truncate max-w-[120px]">
                        {move || username.get()}
                    </span>
                    // The only interactive element here, so the only one
                    // wearing Action Blue.
                    <button
                        data-testid="disconnect"
                        // h-11, not just padding: DESIGN.md's 44px floor applies
                        // to a text button as much as a filled one, and this
                        // one sits right at the edge of the screen.
                        class="h-11 px-2 -mr-2 flex items-center text-caption font-semibold text-action rounded-sm"
                        on:click=disconnect
                    >
                        "Disconnect"
                    </button>
                </div>
            </div>
        </header>
    }
}

/// IANA name plus the offset in effect today — `Asia/Ho_Chi_Minh · UTC+07:00`.
///
/// Every time in this app is a local time, so the footer says which local.
/// Client-side: the server is somewhere else.
#[cfg(feature = "hydrate")]
fn client_timezone() -> Option<String> {
    use wasm_bindgen::JsValue;
    let opts = js_sys::Intl::DateTimeFormat::new(&js_sys::Array::new(), &js_sys::Object::new())
        .resolved_options();
    let name = js_sys::Reflect::get(&opts, &JsValue::from_str("timeZone"))
        .ok()?
        .as_string()?;
    // `wasmbind` makes Local resolve to the browser, so this is today's
    // offset, not the zone's standard one.
    let offset = chrono::Local::now().offset().to_string();
    Some(format!("{name} · UTC{offset}"))
}

/// Footer. DESIGN.md licenses a dense footer, but there is no information
/// architecture here to expose, so density would be decoration.
#[component]
pub fn Footer() -> impl IntoView {
    // Empty on the server, filled after hydration. Rendering it during SSR
    // would print the server's zone and cause a hydration mismatch.
    let timezone = RwSignal::new(String::new());

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(tz) = client_timezone() {
            timezone.set(tz);
        }
    });

    view! {
        <footer data-testid="footer" class="w-full border-t border-hairline mt-auto">
            <div class="max-w-app mx-auto px-4 py-6 flex flex-col gap-1">
                <p class="text-fine text-ink-48">"CalDAV · RFC 4791"</p>
                // Only once known, so the line does not appear then fill in.
                <Show when=move || !timezone.get().is_empty()>
                    <p data-testid="footer-timezone" class="text-fine text-ink-48">
                        {move || timezone.get()}
                    </p>
                </Show>
            </div>
        </footer>
    }
}
