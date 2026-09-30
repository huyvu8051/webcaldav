pub mod calendar_grid;
pub mod connect;
pub mod dav;
pub mod ics;
pub mod layout;
pub mod storage;

use leptos::prelude::*;
use leptos_meta::{MetaTags, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

#[cfg(feature = "ssr")]
pub fn init_executor() {
    any_spawner::Executor::init_tokio().expect("failed to init leptos reactive executor");
}

const TAILWIND_CONFIG: &str = r##"
tailwind.config = {
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        "primary": "#2BE8C9",
        "primary-bright": "#00f5ff",
        "accent-orange": "#FF5500",
        "accent-orange-hover": "#ff6b00",
        "background-dark": "#050508",
        "surface-panel": "#0c151f",
        "surface-inner": "#070c12",
        "border-tech": "#182a3c",
        "text-muted": "#6a8296",
        "text-bright": "#e6f4f1",
      },
      fontFamily: {
        "mono": ["'JetBrains Mono'", "monospace"],
        "display": ["'Public Sans'", "sans-serif"],
        "sans": ["'Public Sans'", "sans-serif"],
        "brand-display": ["'Montserrat'", "sans-serif"]
      },
      borderRadius: {
        "DEFAULT": "1rem",
        "lg": "2rem",
        "xl": "3rem",
        "full": "9999px"
      },
    },
  },
};
"##;

const CAD_GRID_STYLE: &str = r#"
html, body { margin: 0; padding: 0; background-color: #050508; }
.cad-grid-bg {
  background-image:
    linear-gradient(to right, rgba(43, 232, 201, 0.04) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(43, 232, 201, 0.04) 1px, transparent 1px);
  background-size: 28px 28px;
}
.cad-grid-major {
  background-image:
    linear-gradient(to right, rgba(43, 232, 201, 0.09) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(43, 232, 201, 0.09) 1px, transparent 1px);
  background-size: 140px 140px;
}
.bg-blueprint-grid {
  background-size: 28px 28px;
  background-image:
    linear-gradient(to right, rgba(43, 232, 201, 0.05) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(43, 232, 201, 0.05) 1px, transparent 1px);
}
.cyan-glow { box-shadow: 0 0 25px -4px rgba(43, 232, 201, 0.35), 0 0 10px -2px rgba(0, 245, 255, 0.2); }
.cyan-glow-subtle { box-shadow: 0 0 35px -8px rgba(43, 232, 201, 0.16); }
.orange-glow-btn { box-shadow: 0 4px 20px -2px rgba(255, 85, 0, 0.45), 0 0 12px rgba(255, 107, 0, 0.2); }
.orange-glow-btn:hover { box-shadow: 0 6px 28px rgba(255, 85, 0, 0.65); }
"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin=""/>
                <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;600;700&family=Public+Sans:wght@400;500;600;700;800&family=Montserrat:wght@500;600;700&display=swap" rel="stylesheet"/>
                <link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:wght,FILL@100..700,0..1&display=swap" rel="stylesheet"/>
                <script src="https://cdn.tailwindcss.com"></script>
                <script>{TAILWIND_CONFIG}</script>
                <style>{CAD_GRID_STYLE}</style>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body class="bg-background-dark font-mono text-text-bright min-h-screen flex flex-col antialiased">
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    leptos_meta::provide_meta_context();
    view! {
        <Title text="webcaldav"/>
        <Router>
            <Routes fallback=|| "Not found">
                <Route path=StaticSegment("") view=connect::ConnectRoutePage/>
                <Route path=StaticSegment("calendar") view=calendar_grid::CalendarGridRoutePage/>
            </Routes>
        </Router>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
