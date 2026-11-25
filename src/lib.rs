#![recursion_limit = "1024"]

pub mod calendar_grid;
pub mod connect;
pub mod dav;
pub mod ics;
pub mod layout;
pub mod storage;
pub mod ui;

use leptos::prelude::*;
use leptos_meta::{MetaTags, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

#[cfg(feature = "ssr")]
pub fn init_executor() {
    any_spawner::Executor::init_tokio().expect("failed to init leptos reactive executor");
}

// Tokens from DESIGN.md (apple.com, vendored at the repo root). Palette, type
// ladder, radius grammar and the no-shadow rule transfer; its 80px section
// padding and full-bleed tiles do not — this is dense UI, not a landing page.
const TAILWIND_CONFIG: &str = r##"
tailwind.config = {
  theme: {
    extend: {
      colors: {
        // "The single accent is non-negotiable" -- every interactive element
        // is Action Blue and nothing else. Replaces 98 uses of neon teal.
        "action": "#0066cc",
        "action-focus": "#0071e3",
        // Text
        "ink": "#1d1d1f",
        "ink-80": "#333333",
        "ink-48": "#7a7a7a",
        // Surfaces
        "canvas": "#ffffff",
        "parchment": "#f5f5f7",
        "pearl": "#fafafc",
        // Hairlines -- the replacement for every glow in the old theme
        "hairline": "#e0e0e0",
        "divider-soft": "#f0f0f0",
      },
      fontFamily: {
        // The system stack resolves to SF Pro on Apple platforms and to a
        // sane native face everywhere else. No webfont download, which also
        // removes three Google Fonts requests from the critical path.
        "sans": ["-apple-system", "BlinkMacSystemFont", "'SF Pro Text'",
                 "'SF Pro Display'", "system-ui", "'Segoe UI'", "Roboto",
                 "sans-serif"],
      },
      fontSize: {
        // "Run body copy at 17px -- not 16px. The extra pixel defines the
        // brand's reading pace." Tracking is negative throughout.
        "fine":    ["12px", { lineHeight: "1.33", letterSpacing: "-0.12px" }],
        "caption": ["14px", { lineHeight: "1.43", letterSpacing: "-0.224px" }],
        "body":    ["17px", { lineHeight: "1.47", letterSpacing: "-0.374px" }],
        "tagline": ["21px", { lineHeight: "1.38", letterSpacing: "-0.374px" }],
        "title":   ["28px", { lineHeight: "1.14", letterSpacing: "-0.28px" }],
      },
      fontWeight: {
        // 500 is absent on purpose: `font-medium` fails to resolve, which is
        // how a stray one gets caught.
        "light": "300",
        "normal": "400",
        "semibold": "600",
        "bold": "700",
      },
      borderRadius: {
        "sm": "8px",    // compact utility
        "md": "11px",   // pearl capsule
        "lg": "18px",   // cards
        "full": "9999px", // the signature pill -- actions only
      },
      maxWidth: {
        // One layout, mobile-shaped. On a desktop the column stays this wide
        // and the page background simply expands around it, which is the
        // whole point: there is no second layout to keep in step with the
        // first. 430px is the iPhone 15 Pro Max logical width.
        "app": "430px",
      },
    },
  },
};
"##;

// No shadows anywhere: DESIGN.md allows exactly one and reserves it for
// product photography, of which there is none here. Depth is surface-colour
// change plus hairlines.
const BASE_STYLE: &str = r#"
html, body { margin: 0; padding: 0; background-color: #f5f5f7; }
body { -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }

/* "transform: scale(0.95) as the active/press state on every button -- it's
   the system-wide micro-interaction." Applied globally so no button can be
   born without it. */
button:active, a[role="button"]:active { transform: scale(0.95); }
button, a[role="button"] { transition: transform 120ms ease, background-color 120ms ease, color 120ms ease; }

/* The native select arrow sits hard against the right edge, so on a pill it
   reads as falling outside the curve, and each control's width puts it
   somewhere different. Replaced with one chevron on the same 16px inset as
   the text, in Action Blue because it is the affordance. */
select.field-select {
  -webkit-appearance: none;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath d='M1 1.5 6 6.5 11 1.5' fill='none' stroke='%230066cc' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 1rem center;
  background-size: 12px 8px;
  padding-right: 2.5rem;
}
/* Respect the platform preference rather than animating regardless. */
@media (prefers-reduced-motion: reduce) {
  button:active, a[role="button"]:active { transform: none; }
  * { transition-duration: 0.01ms !important; animation-duration: 0.01ms !important; }
}

@keyframes spin { to { transform: rotate(360deg); } }
/* Indeterminate activity indicator (ui.rs). Sized in em and painted in
   currentColor so it belongs to whatever control it sits in. */
.spinner {
  display: inline-block;
  width: 1em;
  height: 1em;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 9999px;
  animation: spin 0.7s linear infinite;
  flex: none;
}
/* The blanket rule above sets every animation to 0.01ms, which would strobe
   this rather than calm it. A still ring still reads as busy. */
@media (prefers-reduced-motion: reduce) {
  .spinner { animation: none !important; opacity: 0.55; }
}
"#;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
                <meta name="theme-color" content="#f5f5f7"/>
                // Three webfont families dropped: the type stack is now the
                // system one, which is SF Pro on Apple platforms and native
                // elsewhere. Material Symbols stays because the UI uses
                // glyphs, and it is the only remaining font request.
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin=""/>
                <link href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,400,0,0&display=swap" rel="stylesheet"/>
                <script src="https://cdn.tailwindcss.com"></script>
                <script>{TAILWIND_CONFIG}</script>
                <style>{BASE_STYLE}</style>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            // The background fills the viewport; the content column does
            // not. That is the entire desktop story.
            <body class="bg-parchment font-sans text-body text-ink min-h-screen flex flex-col">
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
