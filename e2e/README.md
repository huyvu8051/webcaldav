# End-to-end tests

210 tests across 11 specs. `npm run e2e`.

```bash
npm run e2e              # everything, both projects
npm run e2e:phone        # behaviour only, WebKit at iPhone size
npm run e2e:ui           # Playwright's watch mode
npx playwright test -g "delete"   # one slice
```

The dev server starts itself (`webServer` in `playwright.config.ts`) and an
already-running `cargo leptos serve` is reused.

## How it is built

This app has no backend of its own to stub. The WASM client speaks RFC 4791
straight to whatever host the user types, so the only honest seam is the wire,
and `e2e/fixtures/caldav-mock.ts` is a small stateful CalDAV server served
entirely from `page.route`. The real `rsdav`, the real XML parsing and the real
iCalendar handling all run; only the far end is fake.

The request shapes it answers were **captured from the running client**, not
read off the spec. A PUT really changes what the next REPORT returns, so a
save that silently does nothing cannot pass.

Three things the mock can do that a real server will not do on demand:

| | |
|---|---|
| `mock.fail({ match, status })` | make a request fail |
| `mock.hold(match)` | hold a request open until the test releases it |
| `mock.calendars = [...]` | serve exactly the data a test is about |

`hold` is preferred over a delay for observing an in-flight state: a delay is a
bet that the test can look within N milliseconds, and that bet loses under
parallel load.

## Projects

`phone` is WebKit at iPhone size and runs everything — that is the engine and
the viewport this app is built for. `desktop` runs `responsive`,
`design-system` and `a11y` only: the first two because the one thing
specifically about desktop is that the same column sits centred in more
background, and the third because keyboard order cannot be measured on WebKit
(see below). Running the behavioural suite twice mostly bought
engine-specific noise — Chrome renders `accent-color` as #1269c0, for one,
which is not a second accent in the design.

## Accessibility

`a11y.spec.ts` is an axe-core sweep (WCAG 2.1 A/AA plus best-practice) over
five page states plus the error banner, and handwritten tests for what a
scanner cannot see. The sweep runs **unfiltered** and is clean.

Seven gaps were found and all seven are fixed -- see `BACKLOG.md` A1-A7. The
seventh is the instructive one: out-of-month day numbers were `opacity-40` on
`#1d1d1f`, 1.8:1 interactive text, and it only became visible once the
contrast fix let `color-contrast` back into the sweep. An excluded rule hides
more than the thing it was excluded for.

It runs on both projects deliberately: **WebKit will not move focus with Tab**
unless Full Keyboard Access is on, so a keyboard-order assertion there would
pass by never moving at all. Those two tests skip on WebKit and run on
Chromium, which is the honest place to measure them.

One more bad measurement worth remembering: a programmatic `.focus()` does not
set `:focus-visible`, so reading the computed style after one reports "no
outline" for every element on the page. The first version of the focus-ring
test flagged 62 false positives that way. It now presses Tab and reads the
style at each stop.

## Known bugs, tracked as expected failures

Each is a `test.fail` asserting the behaviour that *should* hold, paired with a
passing test asserting what currently happens. They flip green on the day they
are fixed, and the pair catches a change in either direction.

### 1. A failed save is reported as a success — silent data loss

`rsdav` returns `Ok` for a PUT the server rejected, so `create_event` and
`update_event` never reach their error arm. Measured against a 507: one PUT
sent, sheet closed, no error shown, server unchanged. The user types an event,
taps Save, watches the form close, and the event never existed.

`a failed save keeps the form open with what was typed`

### 2. A failed delete is reported as a success

Same root cause, same shape. A 403 on DELETE comes back as `Ok` and nothing is
said. The event survives only because the server refused.

`a failed delete says so and keeps the event`

### 3. An edited event keeps showing its old title

The write lands and the refresh fetches it back — the server really does hold
the new summary and the REPORT really does return it — but the card does not
change. The day list is a `<For>` keyed on `uid`, and each card renders
`event.summary.clone()`, a `String` captured when the row was built. An edit
changes the value, not the key set, so Leptos keeps the existing row and
nothing re-reads it. **Any edit to an existing event is invisible until the key
set changes.**

`an edited event shows its new title`

### 4. A server outage looks exactly like an empty account

`fetch_calendars` answers a 503 with `Ok(vec![])`, so `list_calendars` never
errors. The page shows an empty Calendars panel, no error banner, and no sync
line — pixel-identical to an account that genuinely has no calendars.

`a failed calendar listing is reported, not left blank`

### 5. Service discovery runs twice per page load

`list_calendars` and `list_events_for_calendars` each call `login()`, which
builds a fresh client and redoes the whole `.well-known` → principal →
calendar-home chain. One page load pays for it twice — and so does every save,
every delete and every tab focus, because each of those calls `refresh_events`.
Five wasted round trips per action.

`service discovery runs once per page load, not once per call`

### 6. A nameless calendar renders as `{}`

A calendar whose `displayname` is empty — or absent from the PROPFIND response
entirely — shows the literal characters `{}`. The
`unwrap_or_else(|| "(untitled calendar)")` fallback is unreachable because the
value arrives as `Some("{}")` rather than `None`. Measured both ways.

`a calendar with no name falls back to a readable label`

### 7. The layout breaks when Google Fonts is unreachable

Every glyph is a Material Symbols ligature, so when `fonts.googleapis.com` is
slow, blocked or down, each one renders as its own name in body text:
`chevron_right` is 127px wide instead of a 22px glyph, and the month header
alone pushes the page 26px past the viewport. The same applies to the Tailwind
Play CDN — the page throws `Can't find variable: tailwind` without it.

`the layout survives the icon font failing to load`

### 8. Angle brackets are stripped from a summary

`<` and `>` vanish somewhere between the calendar-data and the card. Not a
security problem — nothing is ever parsed as markup, which is asserted
separately — but a title containing them is silently mangled.

`angle brackets in a summary survive`

## Fixed while writing these

- **Disconnect was a 24px tap target.** Bare text with `px-1 py-0.5`, at the
  edge of the screen, in a 53px bar. Found by the tap-target sweep; now `h-11`.

## Two test bugs worth remembering

Both passed for the wrong reason before they were caught, which is worse than
failing.

- **`new Event('visibilitychange')` does not bubble.** The real event is fired
  at the Document with `bubbles: true`, which is how a window listener sees it.
  A synthetic one defaults to `bubbles: false` and never leaves the document,
  so both focus-refresh tests were asserting against an event nobody received.
- **A click before hydration submits the form natively.** Until the WASM takes
  over, the connect button is a plain submit in a plain form, so an early click
  navigates away and the test watches the wrong page. `waitForHydration` flips
  the password eye, which nothing but the Leptos handler can do.

## Selectors

`data-testid` throughout, 56 of them. Text and glyphs move; these do not. Day
cells also carry `data-date`, `data-today`, `data-selected`, `data-in-month`
and `data-event-count`, so a grid assertion names the day it means instead of
counting squares from a corner.
