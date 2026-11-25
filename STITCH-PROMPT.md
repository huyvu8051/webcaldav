# Stitch prompt

Paste the block below into [stitch.google.com](https://stitch.google.com). It
needs a Google sign-in, which is why this file exists rather than the design
arriving pre-generated.

The prompt carries the whole of `DESIGN.md`'s rule set inline, because Stitch
takes one text box and no attachments — a reference to a file it cannot read
would produce a generically "clean" design rather than this one. The values
below are the same tokens already wired into `src/lib.rs`, so whatever Stitch
returns can be compared against the running app token for token.

---

```
Design a mobile calendar app — a CalDAV client that connects to a user's own
calendar server. Phone only, 430px wide. No tablet or desktop variants.

VISUAL SYSTEM — follow these exactly; they are from apple.com's design system,
not generic "clean minimal".

Colours — one accent and nothing else:
- Action Blue #0066cc for EVERY interactive element: buttons, links, selected
  states, focus rings. No second accent colour anywhere. Not for errors, not
  for destructive actions, not for warnings.
- Text: #1d1d1f primary, #333333 secondary, #7a7a7a muted
- Surfaces: #ffffff cards, #f5f5f7 page background, #fafafc inset
- Hairlines: #e0e0e0 1px borders

Typography — system font (SF Pro / -apple-system):
- Body 17px, line-height 1.47, letter-spacing -0.374px. Seventeen, not sixteen.
- Caption 14px, fine print 12px, section title 21px, screen title 28px
- Weights: 300, 400, 600, 700 only. Never 500.
- Sentence case everywhere. No ALL CAPS. No wide letter-spacing. No monospace.

Shape and depth:
- NO SHADOWS anywhere. None. Depth comes only from surface colour change
  (white card on #f5f5f7 background) and 1px hairline borders.
- NO GRADIENTS. No glows. No decorative background patterns.
- Corner radius: 18px cards, 8px compact controls, full pill for buttons and
  text inputs. Nothing in between.
- Buttons and inputs are 44px tall minimum. Every tap target is 44px.
- Press state: the button scales to 0.95. Nothing else moves.

Spacing: 8px base. 16–24px between elements. This is a dense information UI,
not a marketing page — do not use 80px section padding.

SCREENS

1. Connect screen
   Title "Connect a calendar" and one line of explanation. A white card
   containing: a row of four pill-shaped preset chips (Nextcloud, Radicale,
   iCloud, Fastmail), then three labelled pill inputs — Server address (with a
   small leading server glyph), Username (person glyph), Password (lock glyph,
   with a show/hide eye button inside the field on the right). Below the card,
   a full-width Action Blue pill button reading "Connect". Under everything,
   one line of fine print: credentials stay in this browser.

2. Month screen — the main screen
   A sticky translucent top bar (white at 80% with a backdrop blur, 1px
   hairline underneath): title "Calendar" on the left, the connected username
   quietly on the right, and a "Disconnect" text link in Action Blue.
   Below it: month name and year with previous/next chevrons; a segmented
   control (Month / Week / Timeline — only Month is enabled, the other two are
   visibly disabled); an "Add event" pill button; a row of weekday initials;
   then the month grid.
   The grid is 7 columns of square cells, about 53px each. Each cell shows the
   date number, and up to three small coloured dots underneath for events that
   day. Today's date sits inside a filled Action Blue circle with white text.
   The selected date sits inside a filled dark (#1d1d1f) circle. No borders
   between cells — the circles and the dots carry all the structure.
   Below the grid: a list of the user's calendars, each one a row with a small
   colour swatch, its name, and a checkbox to show or hide it.

3. Day detail — appears under the grid when a date is selected
   The date as a heading, with "3 events" beside it in muted text. Then each
   event as a white card: time, title, and small edit and delete glyph buttons
   on the right. The delete button is NOT red — there is only one accent
   colour, and the glyph itself carries the meaning.

4. Event form
   A sheet with pill inputs for title, start, end; a rounded-18px textarea for
   notes; pill dropdowns for reminder and travel time; a calendar picker
   showing colour swatches. Action Blue pill "Save" at the bottom.

Per-calendar colours are the one place a colour other than Action Blue
appears. They are user-chosen content, not interface chrome — keep them muted
so they sit quietly next to Action Blue rather than competing with it.

Do not add: status indicators, connection badges, version numbers, decorative
icons, illustrations, onboarding screens, or empty-state artwork.
```

---

## After Stitch returns something

Stitch outputs HTML and CSS. It cannot write Leptos, so porting is manual
either way — that is the part the tool does not shorten, and it is most of the
work here (220 `class=` attributes across four files).

What Stitch is genuinely good for is **comparison**: the app has already been
rebuilt against these same tokens by hand, so put the two side by side and
take from Stitch only where it is actually better. Porting its output wholesale
would mean re-deriving a system that already compiles and runs.

Worth checking in whatever it returns, because generators reach for these by
habit and every one of them is wrong here:

- a second accent colour (red for delete, green for success, amber for warnings)
- `box-shadow` on cards or buttons
- a gradient background
- 16px body text
- font-weight 500
- ALL CAPS section headers
