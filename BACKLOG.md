# Backlog

Eight bugs found by the e2e suite on 2026-11-25, none fixed yet. Each one has
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

**Test**: `an edited event shows its new title`.

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

## Already fixed

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
