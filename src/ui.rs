//! Shared feedback primitives.
//!
//! Every network call in this app is a `leptos::prelude::Action`, whose
//! `.pending()` memo is the single source of truth for "in flight". These
//! components are the other half: the one agreed way that fact reaches the
//! screen, so no caller has to invent its own.
//!
//! The shape follows Apple's HIG on progress indicators -- an indeterminate
//! activity indicator shown *at the control that started the work*, never a
//! sheet or overlay covering the screen for an operation this short. The
//! control keeps its size and goes disabled, so nothing reflows under a
//! thumb that is still on the glass.

use leptos::prelude::*;

/// Indeterminate activity indicator. Sized in `em`, so it matches whatever
/// text it sits beside; coloured `currentColor`, so it inherits the button's
/// own foreground instead of introducing a second accent.
#[component]
pub fn Spinner(#[prop(optional, into)] class: String) -> impl IntoView {
    view! { <span class=format!("spinner {class}") aria-hidden="true"></span> }
}

/// A button that cannot be pressed twice and says so.
///
/// `pending` comes straight from an action: `on_click` only has to dispatch,
/// and the in-flight state follows on its own rather than being set and
/// cleared by hand on every branch -- which is how the old code lost it (a
/// save that failed early never reset its flag).
#[component]
pub fn PendingButton(
    #[prop(into)] pending: Signal<bool>,
    /// Resting label.
    #[prop(into)] label: Signal<String>,
    /// In-flight label. Present tense, because it is describing the wait.
    #[prop(into)] busy: Signal<String>,
    #[prop(into)] class: String,
    /// `true` lets a parent `<form>`'s `on:submit` fire. Default is a plain
    /// button that does nothing but call `on_click`.
    #[prop(optional)] submit: bool,
    /// Disabling beyond the pending state -- an unsatisfied form, say.
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <button
            type=move || if submit { "submit" } else { "button" }
            class=class
            // Pressing twice would send a second PUT for the same event.
            disabled=move || pending.get() || disabled.get()
            aria-busy=move || pending.get().to_string()
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            <Show
                when=move || pending.get()
                fallback=move || view! { <span>{move || label.get()}</span> }
            >
                <Spinner/>
                <span>{move || busy.get()}</span>
            </Show>
        </button>
    }
}

/// Glyph-only button that becomes a spinner in place while its own work runs.
///
/// `pending` is per-row, not per-action: one `Action` serves every event in
/// the list, so the caller narrows it with the action's `input()` to the row
/// actually being deleted. Otherwise one tap spins every row at once.
#[component]
pub fn PendingIconButton(
    #[prop(into)] pending: Signal<bool>,
    #[prop(into)] icon: String,
    #[prop(into)] label: String,
    #[prop(into)] class: String,
    #[prop(optional)] on_click: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=class
            aria-label=label
            disabled=move || pending.get()
            aria-busy=move || pending.get().to_string()
            on:click=move |_| {
                if let Some(cb) = on_click {
                    cb.run(());
                }
            }
        >
            <Show
                when=move || pending.get()
                fallback={
                    let icon = icon.clone();
                    move || {
                        view! {
                            <span class="material-symbols-outlined text-[18px]">
                                {icon.clone()}
                            </span>
                        }
                    }
                }
            >
                <Spinner class="text-[18px]"/>
            </Show>
        </button>
    }
}
