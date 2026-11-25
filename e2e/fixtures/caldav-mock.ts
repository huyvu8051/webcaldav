import type { Page, Route } from '@playwright/test'

/**
 * An in-browser CalDAV server.
 *
 * This app has no backend of its own to stub -- the WASM client talks RFC 4791
 * straight to whatever host the user types, so the seam the tests have to take
 * is the wire. Everything here is served from `page.route`, which means the
 * real `rsdav` client, the real XML parsing and the real iCalendar handling
 * all run: only the far end is fake.
 *
 * It is also stateful, because a PUT that does not change what a later REPORT
 * returns would let a broken save pass. And it can be told to fail, stall or
 * return nonsense, which is the half of the behaviour a real server will not
 * reproduce on demand.
 *
 * The request shapes it answers were captured from the running client, not
 * taken from the spec -- see e2e/tests/_discovery.spec.ts.
 */

export const DAV_HOST = 'https://dav.example.test'
export const DAV_BASE = `${DAV_HOST}/`
export const PRINCIPAL = '/principals/alice/'
export const CAL_HOME = '/calendars/alice/'

export const CREDS = { server: DAV_BASE, username: 'alice', password: 'hunter2' }

export interface MockEvent {
  uid: string
  summary: string
  description?: string
  location?: string
  /** `YYYYMMDDTHHMMSS` local, or `YYYYMMDD` when allDay. */
  start: string
  end: string
  allDay?: boolean
  priority?: number
  /** Replaces the generated VEVENT entirely -- for malformed-input tests. */
  rawIcs?: string
}

export interface MockCalendar {
  id: string
  displayName: string
  color?: string
  events: MockEvent[]
}

export interface RecordedRequest {
  method: string
  path: string
  body: string
  depth: string | undefined
}

interface Fault {
  /** Substring matched against `METHOD /path`. */
  match: string
  status?: number
  body?: string
  /** Milliseconds to stall before answering. Pairs with status for both. */
  delayMs?: number
  /** Answer this many times, then stop applying. `undefined` means forever. */
  times?: number
  /** Drop the connection instead of answering. */
  abort?: boolean
}

const XMLNS =
  'xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" ' +
  'xmlns:cs="http://calendarserver.org/ns/" xmlns:ca="http://apple.com/ns/ical/"'

const CORS = {
  'access-control-allow-origin': '*',
  'access-control-allow-headers': '*',
  'access-control-allow-methods':
    'GET,PUT,DELETE,PROPFIND,REPORT,OPTIONS,MKCALENDAR',
  'access-control-expose-headers': 'etag,dav',
}

function pad(ics: string): string {
  return ics.replace(/\n/g, '\r\n')
}

/** Minimal but real VEVENT -- the client parses this for every assertion. */
export function toIcs(ev: MockEvent): string {
  if (ev.rawIcs !== undefined) return pad(ev.rawIcs)
  const dt = ev.allDay
    ? `DTSTART;VALUE=DATE:${ev.start}\nDTEND;VALUE=DATE:${ev.end}`
    : `DTSTART:${ev.start}\nDTEND:${ev.end}`
  return pad(
    [
      'BEGIN:VCALENDAR',
      'VERSION:2.0',
      'PRODID:-//webcaldav-e2e//EN',
      'BEGIN:VEVENT',
      `UID:${ev.uid}`,
      'DTSTAMP:20260101T000000Z',
      dt,
      `SUMMARY:${ev.summary}`,
      ...(ev.description ? [`DESCRIPTION:${ev.description}`] : []),
      ...(ev.location ? [`LOCATION:${ev.location}`] : []),
      ...(ev.priority !== undefined ? [`PRIORITY:${ev.priority}`] : []),
      'END:VEVENT',
      'END:VCALENDAR',
      '',
    ].join('\n'),
  )
}

function xmlEscape(s: string): string {
  return s.replace(/[<>&]/g, (c) => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;' })[c]!)
}

export class CalDavMock {
  calendars: MockCalendar[]
  /** Every request the client made, in order. Assertions read this. */
  readonly requests: RecordedRequest[] = []
  private faults: Fault[] = []
  private gates: { match: string; gate: Promise<void> }[] = []
  /** Bumped on every write, so `getctag` changes the way a real server's does. */
  private ctag = 1
  /** Unhandled requests, surfaced by `assertNoUnhandled` rather than swallowed. */
  readonly unhandled: string[] = []

  /** Real Tailwind and the real icon font. Design and layout specs need both. */
  loadTailwind = false

  constructor(calendars: MockCalendar[] = defaultCalendars()) {
    this.calendars = structuredClone(calendars)
  }

  /** Opt this test into the real Tailwind build, for computed-style checks. */
  withTailwind(): this {
    this.loadTailwind = true
    return this
  }

  /**
   * Holds every matching request open until the returned function is called.
   *
   * Preferred over `delayMs` for observing an in-flight state: a delay is a
   * bet that the test can look within N milliseconds, which is a bet that
   * loses under parallel load. A gate makes the pending state last exactly as
   * long as the assertions need.
   */
  hold(match: string): () => void {
    let release!: () => void
    const gate = new Promise<void>((r) => (release = r))
    this.gates.push({ match, gate })
    return release
  }

  /** Make the next matching request fail. See {@link Fault}. */
  fail(fault: Fault): this {
    this.faults.push(fault)
    return this
  }

  /** Requests matching `METHOD /path`, for counting assertions. */
  matching(substr: string): RecordedRequest[] {
    return this.requests.filter((r) => `${r.method} ${r.path}`.includes(substr))
  }

  countOf(substr: string): number {
    return this.matching(substr).length
  }

  clearRequests(): void {
    this.requests.length = 0
  }

  calendar(id: string): MockCalendar {
    const c = this.calendars.find((c) => c.id === id)
    if (!c) throw new Error(`no such calendar: ${id}`)
    return c
  }

  async install(page: Page): Promise<void> {
    // Tailwind's Play CDN is ~7s of render-blocking JS on a phone and a flaky
    // CDN must not be able to fail a suite about calendars. The shim exists
    // because the page's own inline script assigns `tailwind.config` straight
    // after -- without it the page throws, which is also what happens in
    // production whenever that CDN is slow or blocked.
    // When a test opts into real assets it needs the icon font as well: a
    // stubbed Material Symbols leaves every ligature rendering as its own
    // name, so `chevron_right` is 127px of literal text instead of a 22px
    // glyph -- which is a layout measurement of the stub, not of the app.
    if (this.loadTailwind) {
      // nothing routed: Tailwind and the icon font both load for real
    } else {
      await page.route(/cdn\.tailwindcss\.com/, (route) =>
        route.fulfill({
          status: 200,
          headers: { ...CORS, 'content-type': 'application/javascript' },
          body: 'globalThis.tailwind = { config: {} };',
        }),
      )
      await page.route(/fonts\.(googleapis|gstatic)\.com/, (route) =>
        route.fulfill({ status: 200, body: '', headers: CORS }),
      )
    }
    await page.route(`${DAV_HOST}/**`, (route) => this.handle(route))
  }

  private respond(route: Route, status: number, body = '', extra: Record<string, string> = {}) {
    return route.fulfill({
      status,
      headers: { ...CORS, 'content-type': 'application/xml; charset=utf-8', ...extra },
      body,
    })
  }

  private multistatus(route: Route, inner: string) {
    return this.respond(route, 207, `<?xml version="1.0" encoding="utf-8"?>\n<d:multistatus ${XMLNS}>${inner}</d:multistatus>`)
  }

  private async handle(route: Route): Promise<void> {
    const req = route.request()
    const method = req.method().toUpperCase()
    const path = new URL(req.url()).pathname
    const body = req.postData() ?? ''
    const key = `${method} ${path}`

    if (method === 'OPTIONS') {
      return route.fulfill({ status: 204, headers: { ...CORS, dav: '1, 2, 3, calendar-access' } })
    }

    this.requests.push({ method, path, body, depth: req.headers()['depth'] })

    // Gates come first: a held request has not been answered at all yet, so
    // whatever would have happened still happens, just later.
    const gate = this.gates.find((g) => key.includes(g.match))
    if (gate) await gate.gate

    // Faults are consulted before any real handling, so a test can break a
    // request that would otherwise have succeeded.
    const fault = this.faults.find((f) => key.includes(f.match) && (f.times === undefined || f.times > 0))
    if (fault) {
      if (fault.times !== undefined) fault.times -= 1
      if (fault.delayMs) await new Promise((r) => setTimeout(r, fault.delayMs))
      if (fault.abort) return route.abort('failed')
      if (fault.status) {
        return this.respond(route, fault.status, fault.body ?? `<error>${fault.status}</error>`)
      }
      // delay-only: fall through and answer normally, just late.
    }

    // --- service discovery -------------------------------------------------
    if (path === '/.well-known/caldav') {
      if (method === 'PROPFIND') {
        return this.multistatus(route, `<d:response><d:href>/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`)
      }
      return this.respond(route, 200, '')
    }

    // --- PROPFIND ----------------------------------------------------------
    if (method === 'PROPFIND') {
      if (body.includes('current-user-principal')) {
        return this.multistatus(route, `<d:response><d:href>${path}</d:href><d:propstat><d:prop><d:current-user-principal><d:href>${PRINCIPAL}</d:href></d:current-user-principal></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`)
      }
      if (body.includes('calendar-home-set')) {
        return this.multistatus(route, `<d:response><d:href>${PRINCIPAL}</d:href><d:propstat><d:prop><c:calendar-home-set><d:href>${CAL_HOME}</d:href></c:calendar-home-set></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`)
      }
      // Listing the calendar home: the home itself plus one response per
      // calendar, which is what Depth: 1 means here.
      if (path === CAL_HOME) {
        const home = `<d:response><d:href>${CAL_HOME}</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype><d:displayname>Home</d:displayname></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`
        const cals = this.calendars
          .map(
            (c) =>
              `<d:response><d:href>${CAL_HOME}${c.id}/</d:href><d:propstat><d:prop>` +
              `<d:resourcetype><d:collection/><c:calendar/></d:resourcetype>` +
              `<d:displayname>${xmlEscape(c.displayName)}</d:displayname>` +
              `<cs:getctag>ctag-${this.ctag}</cs:getctag>` +
              `<d:getetag>"ctag-${this.ctag}"</d:getetag>` +
              (c.color ? `<ca:calendar-color>${c.color}</ca:calendar-color>` : '') +
              `<c:supported-calendar-component-set><c:comp name="VEVENT"/></c:supported-calendar-component-set>` +
              `</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`,
          )
          .join('')
        return this.multistatus(route, home + cals)
      }
      // A PROPFIND on one calendar: its objects.
      const own = this.calendarFromPath(path)
      if (own) return this.multistatus(route, this.objectResponses(own, body))
      return this.multistatus(route, '')
    }

    // --- REPORT (calendar-query / calendar-multiget) -----------------------
    if (method === 'REPORT') {
      const cal = this.calendarFromPath(path)
      if (!cal) return this.respond(route, 404)
      return this.multistatus(route, this.objectResponses(cal, body))
    }

    // --- object writes ------------------------------------------------------
    const objectMatch = path.match(/^\/calendars\/alice\/([^/]+)\/(.+)\.ics$/)
    if (objectMatch) {
      const [, calId, uid] = objectMatch
      const cal = this.calendars.find((c) => c.id === calId)
      if (!cal) return this.respond(route, 404)

      if (method === 'PUT') {
        const parsed = parseVEvent(body, uid)
        const existing = cal.events.findIndex((e) => e.uid === parsed.uid)
        if (existing >= 0) cal.events[existing] = parsed
        else cal.events.push(parsed)
        this.ctag += 1
        return this.respond(route, existing >= 0 ? 204 : 201, '', { etag: `"etag-${this.ctag}"` })
      }
      if (method === 'DELETE') {
        const before = cal.events.length
        cal.events = cal.events.filter((e) => e.uid !== uid)
        this.ctag += 1
        return this.respond(route, before === cal.events.length ? 404 : 204)
      }
      if (method === 'GET') {
        const ev = cal.events.find((e) => e.uid === uid)
        if (!ev) return this.respond(route, 404)
        return this.respond(route, 200, toIcs(ev), { 'content-type': 'text/calendar' })
      }
    }

    this.unhandled.push(`${key}\n${body.slice(0, 400)}`)
    return this.respond(route, 404)
  }

  private calendarFromPath(path: string): MockCalendar | undefined {
    const m = path.match(/^\/calendars\/alice\/([^/]+)\/?$/)
    return m ? this.calendars.find((c) => c.id === m[1]) : undefined
  }

  private objectResponses(cal: MockCalendar, _body: string): string {
    return cal.events
      .map(
        (ev) =>
          `<d:response><d:href>${CAL_HOME}${cal.id}/${ev.uid}.ics</d:href><d:propstat><d:prop>` +
          `<d:getetag>"etag-${ev.uid}-${this.ctag}"</d:getetag>` +
          `<c:calendar-data>${xmlEscape(toIcs(ev))}</c:calendar-data>` +
          `</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>`,
      )
      .join('')
  }
}

/** Round-trips a PUT body back into the mock's own model. */
function parseVEvent(ics: string, fallbackUid: string): MockEvent {
  const get = (name: string): string | undefined => {
    const m = ics.match(new RegExp(`^${name}(?:;[^:\r\n]*)?:(.*)$`, 'mi'))
    return m ? m[1].trim() : undefined
  }
  const startLine = ics.match(/^DTSTART(;[^:\r\n]*)?:(.*)$/mi)
  const endLine = ics.match(/^DTEND(;[^:\r\n]*)?:(.*)$/mi)
  const allDay = !!startLine?.[1]?.includes('VALUE=DATE')
  const priority = get('PRIORITY')
  return {
    uid: get('UID') ?? fallbackUid,
    summary: get('SUMMARY') ?? '',
    description: get('DESCRIPTION'),
    location: get('LOCATION'),
    start: startLine?.[2]?.trim() ?? '',
    end: endLine?.[2]?.trim() ?? '',
    allDay,
    priority: priority ? Number(priority) : undefined,
  }
}

/** Dates are computed from the run date so "today" is always inside them. */
export function ymd(offsetDays = 0): string {
  const d = new Date()
  d.setDate(d.getDate() + offsetDays)
  return `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, '0')}${String(d.getDate()).padStart(2, '0')}`
}

export function defaultCalendars(): MockCalendar[] {
  return [
    {
      id: 'work',
      displayName: 'Work',
      color: '#8a9a7b',
      events: [
        { uid: 'ev-standup', summary: 'Standup', start: `${ymd(0)}T090000`, end: `${ymd(0)}T091500` },
        { uid: 'ev-review', summary: 'Design review', description: 'Bring the mocks', location: 'Room 2', start: `${ymd(0)}T140000`, end: `${ymd(0)}T150000` },
        { uid: 'ev-1on1', summary: 'One on one', start: `${ymd(1)}T110000`, end: `${ymd(1)}T113000` },
      ],
    },
    {
      id: 'personal',
      displayName: 'Personal',
      color: '#7b8a9a',
      events: [
        { uid: 'ev-dentist', summary: 'Dentist', start: `${ymd(2)}T080000`, end: `${ymd(2)}T084500` },
      ],
    },
  ]
}

/** Puts credentials in localStorage so a test can start on /calendar. */
export async function seedSession(page: Page): Promise<void> {
  await page.addInitScript((creds) => {
    window.localStorage.setItem(
      'webcaldav.connection',
      JSON.stringify({ server_url: creds.server, username: creds.username, password: creds.password }),
    )
  }, CREDS)
}
