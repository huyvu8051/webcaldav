# Backlog

Sixteen bugs found by the e2e suite on 2026-11-25: ten functional, still
open, and seven accessibility, now fixed. Each open one has
a `test.fail` in `e2e/` asserting the behaviour that *should* hold, paired with
a passing test pinning what currently happens — so a fix flips the pair and a
regression flips it back.

`e2e/README.md` describes the symptoms. This file is the plan.

Run `npm run e2e` to see where things stand.

---

## Order to do them in

1, 2 and 3 are user-visible data problems and should go first. 5 is a few
lines and removes most of the app's latency. 4, 6, 7, 8 are smaller.

| # | What | Where the fix lives | Size |
|---|---|---|---|
| 1 | Failed save reported as success | `rsdav` | medium |
| 2 | Failed delete reported as success | `rsdav` | (same fix as 1) |
| 3 | Edited event keeps its old title | `calendar_grid.rs` | one line |
| 4 | Server outage looks like an empty account | `rsdav` | (same fix as 1) |
| 5 | Service discovery repeats on every call | `dav.rs` | small |
| 6 | Nameless calendar renders as `{}` | `rsdav`, or band-aid here | small |
| 7 | Layout breaks without Google Fonts / Tailwind CDN | `lib.rs` + `Dockerfile` | medium |
| 8 | Angle brackets stripped from a summary | `rsdav` parse path | small |
| **S1** | **A page that cannot hydrate looks signed in and cannot be left** | `lib.rs` + `deployment.yaml` | **medium, do first** |
| S2 | A corrupt stored config is left in localStorage | `storage.rs` | one line |
| S3 | 9.4 MB wasm — ~2s of blank before hydration even on a good connection | `Cargo.toml` + `Dockerfile` | medium |
| S4 | The local dev server serves a debug wasm, so it misrepresents speed | *(use `--release`)* | note |

---

## S1 — a page that cannot hydrate looks signed in, and traps you

The symptom, reproduced three ways: you are on `/calendar`, **the name never
fills in, Disconnect does nothing, no calendars load, and no error appears**.
The only way out is clearing site data by hand.

Partly improved since: the page now sits on "Updating…" with a name skeleton,
so it reads as *stuck* rather than as a finished empty account. That removes
the lie but not the dead end.

`username`, the CalDAV config and the Disconnect handler are all set inside
the `#[cfg(feature = "hydrate")]` effect in `calendar_grid.rs`. If the WASM
never runs, the server-rendered markup stays on screen — including a
Disconnect button that was never wired to anything.

**This is not hypothetical, and the trigger is our own rollout.** Measured:

- each pod bakes in exactly one content-hashed bundle
  (`COPY --from=wasm-builder /app/pkg /pkg`, `LEPTOS_OUTPUT_NAME` read from
  `/output-name.txt` at startup), and serves **404** for any other hash —
  confirmed against the live site: the current hash returns 200, a made-up one
  returns 404, with no fallback
- `webcaldav-service` has `sessionAffinity: None` and the ingress has no
  sticky-session annotation
- the deployment runs `replicas: 2` with `maxSurge: 1`

So during a rollout the document can come from the new pod while the browser's
follow-up request for `/pkg/webcaldav-<newhash>.js` round-robins to the old
pod, which does not have it. That exact 50/50 split across pods was observed
while deploying `c73c57c`.

Cloudflare is not caching the HTML (`cf-cache-status: DYNAMIC`), so the edge
is not the problem — the two pods are.

**Fix, two halves:**

1. *Stop the trigger.* Keep a client's document and its bundle on one pod:
   `sessionAffinity: ClientIP` on the Service, or a sticky-session cookie
   annotation on the ingress. Either closes the window without touching the
   app.
2. *Recover when it happens anyway* — a dead bundle has other causes. Add an
   inline `<script>` in the shell (plain, not a module, so it runs even when
   the bundle is gone) that sets a timer and, if hydration has not signalled
   within a few seconds, reveals a banner: "could not load" plus a button that
   does `localStorage.clear(); location.href = '/'`. That turns a trap into a
   message.

Optionally a third: do not render the signed-in top bar server-side at all, so
a page that never hydrates does not *claim* to be signed in. That stops it
lying but does not unstick anyone, so it is the least important of the three.

**Tests**: `e2e/tests/session-recovery.spec.ts` — five tests pin the current
behaviour across a 404'd wasm, a 404'd script and a script that throws, and
`a page that cannot hydrate says so` is the tracked failure that flips when
this is fixed.

**Already correct, and now guarded**: an unreachable server, a 401 after the
fact, a corrupt stored config and storage blocked entirely all behave properly
— the username shows, the error shows, and Disconnect works.

---

## S3 — the bundle is 9.4 MB, and hydration is the slow part

Measured against the deployed build, from a machine on a good connection:

| | |
|---|---|
| HTML painted | 566 ms |
| wasm request starts | 584 ms |
| wasm finished arriving | 2347 ms (1.02 MB on the wire) |
| **username appears** | **2498 ms** |

So roughly **two seconds of a page that is fully painted but not yet alive**,
and that is the good case. A `curl` of the same file over the tunnel took
**5.6 s** at 186 KB/s, and a phone on mobile data will be worse. On localhost,
where the download is 30 ms, it still takes **1.1 s** — that part is the
browser compiling and instantiating the module, and no amount of CDN tuning
removes it.

Compression is already fine: Cloudflare serves it `zstd`, 9.38 MB → 1.02 MB.
The problem is the 9.4 MB input — and the cause is now known.

**The Dockerfile never runs `wasm-opt`.** It hand-rolls the wasm step --
`cargo build --release --target wasm32-unknown-unknown` then `wasm-bindgen`
(lines 24 and 34) -- which bypasses `cargo-leptos`, and `cargo-leptos` is what
normally runs `wasm-opt` on a release build. Measured on the same commit:

| build | wasm |
|---|---|
| `cargo leptos serve` (debug) | 7.8 MB |
| `cargo leptos serve --release` | **2.0 MB** |
| the deployed image | **9.4 MB** |

So production ships a bundle **4.7× larger than the same code built locally in
release**, and the whole gap is one missing optimiser pass. Hydration measured
against the local release build is **103 ms**, against the local debug build
**1128 ms**.

**Fixed for now** by covering the window honestly — the server-rendered page
shows "Updating…", a spinner and a name skeleton, so the wait is visible
instead of looking like a finished empty calendar. That is a dressing, not a
cure.

**Actually fixing it**, in order of return:

1. **Run `wasm-opt -Oz` in the `wasm-builder` stage**, after `wasm-bindgen`.
   On the evidence above this is most of the 9.4 → ~2 MB on its own, and it is
   a two-line change. (Or stop hand-rolling and let `cargo-leptos` do the
   build, which is what it is for -- but the readme explains why that was
   avoided, so the targeted fix is safer.)
2. Add a `[profile.release]` to `Cargo.toml`; there is none at all today.
   `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`,
   `strip = true`.
3. Only then go looking at what is pulling weight -- `chrono` with full locale
   data and `serde_json` are the usual suspects. `twiggy top` will say rather
   than guess.

Worth doing before S1's affinity fix, since a smaller bundle shortens the
window S1 is about.

## S4 — a debug dev server makes the app feel broken on a phone

**Confirmed 2026-11-25.** A laggy-feeling day grid on an iPhone was the debug
build, nothing else. Rebuilt with `--release` and it was smooth; same commit,
same device, same network.

`cargo leptos serve` builds in debug and serves 7.8 MB of unoptimised wasm.
`--release` serves 2.0 MB. Measured against each, on the same machine:

| | debug | release |
|---|---|---|
| wasm | 7.8 MB | 2.0 MB |
| time to hydrate | 1128 ms | **103 ms** |
| tap latency (desktop CPU) | 13 ms | 13 ms |

The tap number is the interesting one: **on a desktop CPU there is no
difference at all**, which is exactly why the problem was invisible from here
and obvious on the phone. A phone has to parse, compile and run four times
more unoptimised code, and that is where the 11× hydration gap comes from.

**So: always test speed against `cargo leptos serve --release`.** A debug
build is fine for behaviour and misleading for anything else.

### What this was *not*

`touch-action: manipulation` was added in the same pass, on the theory that
Safari was holding clicks back for double-tap-zoom. It was a reasonable
hypothesis and it is correct practice, so it stays — but it was **not** the
cause and it is not what fixed this. Recording that, because a line of CSS
sitting next to a resolved bug reads like the cure a year later.

Related to S3: the deployed image is 9.4 MB — *release*, but built without
`wasm-opt`, so it is worse than the 2.0 MB a local release build produces.

## S2 — a corrupt stored config is left behind

`storage::load()` returns `None` for unparseable JSON and the page correctly
redirects to connect, but the unparseable value stays in localStorage, so
every later visit repeats the same round trip.

**Fix** — clear the key when parsing fails:

```rust
match serde_json::from_str(&raw) {
    Ok(cfg) => Some(cfg),
    Err(_) => { let _ = storage.remove_item(STORAGE_KEY); None }
}
```

**Test**: `a corrupt stored config is cleared, not left behind`.

---

## 1, 2, 4 — non-2xx responses come back as `Ok`

One root cause, three symptoms. `rsdav` is not checking HTTP status before
returning:

- a PUT the server rejected (measured with 507) returns `Ok`, so
  `create_event` / `update_event` in `src/dav.rs` never reach their error arm.
  The sheet closes, nothing is reported, and the event was never written.
- a DELETE rejected with 403 does the same.
- `fetch_calendars` answers a 503 with `Ok(vec![])`, so `list_calendars`
  cannot tell an outage from an empty account.

**Fix** — in `huyvu8051/rsdav`, every request helper must treat a non-2xx
response as an error rather than an empty success. Start from whatever wraps
the `fetch` call and check `response.status()` before parsing; a 207
multistatus additionally needs its per-href `<d:status>` checked, since a
multistatus can be 207 overall while individual hrefs failed.

**Reproduce** without the suite:

```ts
mock.fail({ match: 'PUT', status: 507 })   // e2e/fixtures/caldav-mock.ts
```

**Interim mitigation if rsdav cannot change soon**: `save` already calls
`refresh_events()` on success. It could compare the refreshed list against
what it just wrote and surface "the server did not accept this" when the event
is missing. That is a workaround, not a fix — it cannot distinguish a rejected
write from a server that is merely slow to make it visible.

**Tests**: `a failed save keeps the form open with what was typed`,
`a failed delete says so and keeps the event`,
`a failed calendar listing is reported, not left blank`.

---

## 3 — an edited event keeps showing its old title

`src/calendar_grid.rs:893`:

```rust
key=|e| e.uid.clone()
```

Each card renders `event.summary.clone()` — a `String` captured when the row
was built. An edit changes the value but not the key set, so Leptos keeps the
existing row and nothing ever re-reads it. The write lands and the refetch
returns the new data; only the DOM is stale.

**Fix** — key on something that changes when the event changes. `EventSummary`
already carries `etag` straight off the wire (`dav.rs:113`), and RFC 4918
requires a server to change it whenever the resource changes:

```rust
key=|e| (e.uid.clone(), e.etag.clone())
```

That rebuilds the row on edit and leaves untouched rows alone. It does mean a
server with broken etags would never refresh a row, so if that turns out to be
a problem the alternative is to hold each event in a signal and render from
it, which is more code but does not depend on the server.

Worth checking the calendars `<For>` at `:834` (`key=|c| c.url.clone()`) at the
same time — a renamed or recoloured calendar has the same shape of problem.

**Scope, narrowed by `dates.spec.ts`**: only edits that leave the event on
the same day are invisible. Moving one to another day or month does show,
because the day list is rebuilt from a different source when the selection
changes — so the key set really does change. That makes this easy to miss in
manual testing: rename an event in place and it looks broken, move it and it
looks fine.

**Test**: `an edited event shows its new title`, with
`an event moved to another month leaves the old day and arrives on the new`
pinning the case that already works.

---

## 5 — service discovery repeats on every call

`src/dav.rs` calls `login(config)` at five sites — lines 71, 106, 172, 187 and
206 — and each one builds a fresh client and redoes the whole
`.well-known/caldav` → `current-user-principal` → `calendar-home-set` chain
before doing any real work.

Measured on one page load: the chain runs twice. A save runs it again for the
PUT and again for the refresh that follows. On a server with 100ms RTT that is
roughly half a second of pure overhead per action.

**Fix** — build the client once and reuse it. A `thread_local!` cache in
`dav.rs` keyed on the `ConnectionConfig` is enough (this is WASM, single
threaded), or hold an `Rc<DavClient>` in a signal alongside `config` in
`calendar_grid.rs`. Invalidate on disconnect and on a 401.

**Test**: `service discovery runs once per page load, not once per call` —
currently asserts `.well-known` is requested exactly once; the paired passing
test records that it is currently twice.

---

## 6 — a nameless calendar renders as `{}`

A calendar whose `displayname` is empty, or absent from the PROPFIND response
entirely, shows the literal characters `{}` in the Calendars panel. The
fallback in `dav.rs`:

```rust
display_name: c.display_name.unwrap_or_else(|| "(untitled calendar)".to_string()),
```

is unreachable, because the value arrives as `Some("{}")` rather than `None` —
something upstream is rendering an empty value through a `Debug`/serde path.
Measured both with an empty element and with the element omitted.

**Fix** — properly, in `rsdav`: an absent or empty `displayname` should be
`None`. **Band-aid here** meanwhile:

```rust
display_name: c.display_name
    .filter(|n| !n.trim().is_empty() && n != "{}")
    .unwrap_or_else(|| "(untitled calendar)".to_string()),
```

Tracking down why it is `{}` is worth doing before reaching for the band-aid —
if `Debug` output is leaking into one string field it is probably leaking into
others, and #8 may be the same leak.

**Test**: `a calendar with no name falls back to a readable label`.

---

## 7 — the page depends on two CDNs it cannot reach offline

Two separate fragilities, same cause:

- **Icon font.** Every glyph is a Material Symbols ligature. When
  `fonts.googleapis.com` is slow, blocked or down, each renders as its own
  name in body text — `chevron_right` is 127px wide instead of a 22px glyph —
  and the month header alone pushes the page 26px past a 390px viewport.
- **Tailwind.** `lib.rs:155` loads `https://cdn.tailwindcss.com` and the
  inline script immediately after assigns `tailwind.config`. Without the CDN
  the page throws `Can't find variable: tailwind`, and the Play build is ~7s
  of render-blocking JS on a phone even when it does load.

**Fix**:

- Self-host the glyphs. Either ship the Material Symbols woff2 from `/pkg`, or
  replace the dozen or so icons actually used with an inline SVG sprite, which
  also removes the ligature-name failure mode entirely.
- Compile Tailwind ahead of time in the `Dockerfile` instead of the Play CDN.
  The other repo already did this for the same reason.
- Until then, at minimum guard the inline config (`if (window.tailwind)`) so a
  missing CDN does not throw.

**Tests**: `the layout survives the icon font failing to load`, and the paired
`without the icon font the page currently overflows by 26px`.

---

## 8 — angle brackets stripped from a summary

A summary of `<img src=x onerror=alert(1)>` reaches the card as
`img src=x onerror=alert(1)`. Not a security problem — nothing is ever parsed
as markup, and that is asserted separately and passing — but a title
containing `<` or `>` is silently mangled.

**Fix** — find where it happens. The mock sends the summary XML-escaped inside
`<c:calendar-data>`, so the loss is in the XML unescape or in the iCalendar
line parse downstream of it. Possibly the same leak as #6.

**Test**: `angle brackets in a summary survive`.

---

## A1-A7 — accessibility, fixed 2026-11-25

Found by `e2e/tests/a11y.spec.ts`: an axe-core sweep (WCAG 2.1 A/AA plus
best-practice) over five page states plus the error banner, and handwritten
tests for the things a scanner cannot see. The sweep now runs **unfiltered**
on both projects and comes back clean.

| | Was | Now |
|---|---|---|
| A1 | Five form selects had no accessible name (axe: critical) | `aria-label` on all seven selects |
| A2 | `#7a7a7a` on `#f5f5f7` was 3.94:1, AA wants 4.5 | token is `#6e6e73` — 4.66 / 5.07 / 4.86 on parchment / canvas / pearl |
| A3 | A day cell announced only `"25, button"` | `aria-label` with weekday, date, "today" and the event count, plus `aria-current="date"` |
| A4 | No live region anywhere; a failed save was silent | `role="alert"` on both error banners, `aria-live="polite"` on the sync line |
| A5 | One heading on the whole calendar page | month title, "Calendars" and the selected date are `h2`; no skipped levels |
| A6 | Month disclosure had no `aria-controls` | points at `#month-picker`, which is always rendered so the idref never dangles |
| A7 | *(surfaced by the fix)* out-of-month day numbers were `opacity-40` on `#1d1d1f` — 1.8:1 interactive text | dimmed by colour (`ink-48`) instead; the dots keep the opacity, since they carry no text |

A7 only became visible once A2 was done and `color-contrast` could come back
into the sweep — it had been hidden behind the rule exclusion the whole time.
Worth remembering: an excluded rule hides more than the thing it was excluded
for.

On A2 specifically: DESIGN.md scopes `ink-muted-48` to "disabled button text
and legal fine-print", where WCAG exempts contrast. This app had been using it
for weekday initials, event times, the connected username and the sync line,
which are information rather than decoration. `#6e6e73` is Apple's own
secondary-label grey, so the palette stays in the family rather than drifting
to an arbitrary darker grey.

**Also now locked in**, because it is the half that silently rots: tab order
is the reading order; day cells are real `<button>`s so Enter and Space both
work; every tabbed control shows a focus indicator; every icon-only button has
a label; `aria-expanded`, `aria-pressed` and `aria-busy` are correct; the
spinner is `aria-hidden`; `lang`, `<title>` and the landmarks are present.

## Already fixed## Already fixed

- **Disconnect was a 24px tap target.** Bare text with `px-1 py-0.5`, at the
  edge of the screen in a 53px bar, against DESIGN.md's 44px floor. Found by
  the tap-target sweep; now `h-11`. Guarded by
  `Disconnect is a full-height target, not a 24px one`.

## Deploy state

`webcaldav.opendiy.vn` is serving `c73c57c` (the apple.com redesign), verified
after rolling both pods: 8/8 requests returned the new build and ArgoCD stayed
`Synced/Healthy`.

`4f0c177` — this suite, the testids and the Disconnect fix — is built and in
GHCR but **not rolled out**. To ship it:

```bash
# Not `kubectl rollout restart`: the annotation it stamps is drift in a
# selfHeal namespace and gets reverted. Deleting pods is not drift, and
# imagePullPolicy: Always makes the replacements pull the fresh :latest.
kubectl delete pod <name> -n webcaldav --wait=true   # one at a time
curl -s https://webcaldav.opendiy.vn/ | grep -c field-select   # verify
```
