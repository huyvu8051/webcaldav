import { test, expect, isoDay, dayCell, fillNewEvent, localDateTime, waitForIdle } from '../fixtures/app'

test.describe('creating, editing and deleting events', () => {
  test('the form is closed until Add event is tapped', async ({ connected: page }) => {
    // Regression: the form used to live in a desktop sidebar and was always
    // rendered, so once the columns collapsed there was an Add button sitting
    // on top of an already-open form.
    await expect(page.getByTestId('event-form')).toHaveCount(0)
    await expect(page.getByTestId('add-event')).toBeVisible()
    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('event-form')).toBeVisible()
    await expect(page.getByTestId('add-event')).toHaveCount(1)
  })

  test('a new event is PUT to the server and shows up on the day', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await fillNewEvent(page, {
      summary: 'Lunch with Mai',
      start: localDateTime(0, 12),
      end: localDateTime(0, 13),
      location: 'Cafe',
    })
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('event-form')).toHaveCount(0)
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Lunch with Mai' })).toBeVisible()

    const put = mock.matching('PUT /calendars/alice/')
    expect(put).toHaveLength(1)
    expect(put[0].body).toContain('SUMMARY:Lunch with Mai')
    expect(put[0].body).toContain('LOCATION:Cafe')
    expect(put[0].body).toMatch(/BEGIN:VCALENDAR[\s\S]*END:VCALENDAR/)
    // The mock is stateful, so the event really exists server-side now.
    expect(mock.calendar('work').events.map((e) => e.summary)).toContain('Lunch with Mai')
  })

  test('the new event goes to the calendar chosen in the form', async ({ mock, page, connected }) => {
    void connected
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-calendar-url').selectOption({ label: 'Personal' })
    await page.getByTestId('form-summary').fill('Haircut')
    await page.getByTestId('form-start').fill(localDateTime(0, 16))
    await page.getByTestId('form-end').fill(localDateTime(0, 17))
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('event-form')).toHaveCount(0)
    expect(mock.matching('PUT /calendars/alice/personal/')).toHaveLength(1)
    expect(mock.calendar('personal').events.map((e) => e.summary)).toContain('Haircut')
  })

  test('the form reopens empty after a successful save', async ({ connected: page }) => {
    await fillNewEvent(page, { summary: 'One', start: localDateTime(0, 9), end: localDateTime(0, 10) })
    await page.getByTestId('form-submit').click()
    await expect(page.getByTestId('event-form')).toHaveCount(0)

    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-summary')).toHaveValue('')
  })

  // KNOWN BUG, and the worst one here. `rsdav` returns Ok for a PUT that the
  // server rejected, so `create_event`/`update_event` never reach their error
  // arm: the sheet closes, no error appears, and the event was never written.
  // Measured against a 507: 1 PUT sent, form closed, no error, server
  // unchanged. Silent data loss.
  test.fail('a failed save keeps the form open with what was typed', async ({ mock, page, connected }) => {
    void connected
    mock.fail({ match: 'PUT', status: 507 })
    await fillNewEvent(page, { summary: 'Should survive', start: localDateTime(0, 9), end: localDateTime(0, 10) })
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('status-error')).toBeVisible()
    await expect(page.getByTestId('status-error')).toContainText(/save/i)
    // Dismissing the sheet here would throw away the user's typing along
    // with it, which is why the form only closes on success.
    await expect(page.getByTestId('event-form')).toBeVisible()
    await expect(page.getByTestId('form-summary')).toHaveValue('Should survive')
    await expect(page.getByTestId('form-submit')).toBeEnabled()
  })

  test('editing fills the form from the event and relabels the button', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    const card = page.locator('[data-testid="event-card"]').filter({ hasText: 'Design review' })
    await card.getByTestId('event-edit').click()

    await expect(page.getByTestId('event-form')).toBeVisible()
    await expect(page.getByTestId('form-summary')).toHaveValue('Design review')
    await expect(page.getByTestId('form-description')).toHaveValue('Bring the mocks')
    await expect(page.getByTestId('form-location')).toHaveValue('Room 2')
    await expect(page.getByTestId('form-submit')).toContainText('Save')
    await expect(page.getByTestId('form-cancel')).toBeVisible()
  })

  test('an edit writes to the same object rather than creating a second', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    const before = mock.calendar('work').events.length
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-edit').click()
    await page.getByTestId('form-summary').fill('Standup (moved)')
    await page.getByTestId('form-submit').click()
    await waitForIdle(page)

    // The write itself is correct: same href, same UID, new summary.
    expect(mock.calendar('work').events).toHaveLength(before)
    expect(mock.calendar('work').events.find((e) => e.uid === 'ev-standup')?.summary).toBe('Standup (moved)')
    expect(mock.matching('PUT /calendars/alice/work/ev-standup.ics')).toHaveLength(1)
  })

  // KNOWN BUG. The write lands and the refresh fetches it back -- the server
  // really does hold "Standup (moved)" and the REPORT really does return it --
  // but the card keeps showing the old title. The day list is a `<For>` keyed
  // on `uid`, and each card renders `event.summary.clone()`, a String captured
  // when the row was built. An edit changes the value, not the key set, so
  // Leptos keeps the existing row and nothing re-reads the summary. Any edit
  // to an existing event is invisible until the key set changes.
  test.fail('an edited event shows its new title', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-edit').click()
    await page.getByTestId('form-summary').fill('Standup (moved)')
    await page.getByTestId('form-submit').click()
    await waitForIdle(page)

    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup (moved)' })).toBeVisible()
  })

  test('an edited event currently keeps showing its old title', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-edit').click()
    await page.getByTestId('form-summary').fill('Standup (moved)')
    await page.getByTestId('form-submit').click()
    await waitForIdle(page)

    expect(mock.calendar('work').events.find((e) => e.uid === 'ev-standup')?.summary).toBe('Standup (moved)')
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).first()).toBeVisible()
  })

  test('Add event after an edit creates rather than overwrites', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-edit').click()
    await page.getByTestId('form-cancel').click()

    await fillNewEvent(page, { summary: 'Brand new', start: localDateTime(0, 18), end: localDateTime(0, 19) })
    await page.getByTestId('form-submit').click()
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Brand new' })).toBeVisible()
    // Still there: the edit was abandoned, not applied to the new event.
    expect(mock.calendar('work').events.find((e) => e.uid === 'ev-standup')?.summary).toBe('Standup')
  })

  test('picking a different day does not wipe the open form', async ({ connected: page }) => {
    // select_day used to call form.clear(), set the calendar and re-date the
    // form -- all of which open_new_event already does. The visible cost was
    // that tapping a date while the form was open silently emptied it.
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-summary').fill('Half-typed title')
    await page.getByTestId('form-location').fill('Somewhere')

    await dayCell(page, isoDay(3)).click()
    await expect(dayCell(page, isoDay(3))).toHaveAttribute('data-selected', 'true')
    await expect(page.getByTestId('event-form')).toBeVisible()
    await expect(page.getByTestId('form-summary')).toHaveValue('Half-typed title')
    await expect(page.getByTestId('form-location')).toHaveValue('Somewhere')
  })

  test('tapping a day makes no network request', async ({ mock, page, connected }) => {
    void connected
    // Every event is already in memory -- there is no time-range filter on
    // the fetch -- so selection is pure local filtering. If this ever starts
    // issuing a REPORT, the grid has become a per-day fetch by accident.
    mock.clearRequests()
    for (const d of [0, 1, 3, 5, 8]) await dayCell(page, isoDay(d)).click()
    await page.waitForTimeout(800)
    expect(mock.requests).toEqual([])
  })

  test('Add event after picking a day still dates itself to that day', async ({ connected: page }) => {
    // The slimming above is only safe because open_new_event redoes the work.
    await dayCell(page, isoDay(6)).click()
    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-start')).toHaveValue(new RegExp('^' + isoDay(6)))
    await expect(page.getByTestId('form-summary')).toHaveValue('')
  })

  test('Cancel closes the form and clears it', async ({ connected: page }) => {
    await dayCell(page, isoDay(0)).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-edit').click()
    await page.getByTestId('form-cancel').click()
    await expect(page.getByTestId('event-form')).toHaveCount(0)

    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-summary')).toHaveValue('')
    await expect(page.getByTestId('form-cancel')).toHaveCount(0)
  })

  test('deleting removes the event from the server and the list', async ({ mock, page, connected }) => {
    void connected
    await dayCell(page, isoDay(0)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(2)

    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-delete').click()
    await expect(page.getByTestId('event-card')).toHaveCount(1)
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' })).toHaveCount(0)

    expect(mock.matching('DELETE /calendars/alice/work/ev-standup.ics')).toHaveLength(1)
    expect(mock.calendar('work').events.map((e) => e.uid)).not.toContain('ev-standup')
  })

  // KNOWN BUG, same root cause as the failed save: a 403 on DELETE comes back
  // as Ok, so nothing is reported. The event does survive -- but only because
  // the server refused, not because the app noticed.
  test.fail('a failed delete says so and keeps the event', async ({ mock, page, connected }) => {
    void connected
    mock.fail({ match: 'DELETE', status: 403 })
    await dayCell(page, isoDay(0)).click()
    await page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' }).getByTestId('event-delete').click()

    await expect(page.getByTestId('status-error')).toBeVisible()
    await expect(page.getByTestId('status-error')).toContainText(/delete/i)
    await expect(page.locator('[data-testid="event-card"]').filter({ hasText: 'Standup' })).toBeVisible()
  })

  test('deleting the last event on a day empties its dots', async ({ connected: page }) => {
    await dayCell(page, isoDay(2)).click()
    await expect(page.getByTestId('event-card')).toHaveCount(1)
    await page.getByTestId('event-delete').click()
    await expect(page.getByTestId('event-card')).toHaveCount(0)
    await expect(dayCell(page, isoDay(2))).toHaveAttribute('data-event-count', '0')
  })

  test('the all-day switch changes the input type on both ends', async ({ connected: page }) => {
    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-start')).toHaveAttribute('type', 'datetime-local')
    await expect(page.getByTestId('form-end')).toHaveAttribute('type', 'datetime-local')

    await page.getByTestId('form-all-day').check()
    await expect(page.getByTestId('form-start')).toHaveAttribute('type', 'date')
    await expect(page.getByTestId('form-end')).toHaveAttribute('type', 'date')

    await page.getByTestId('form-all-day').uncheck()
    await expect(page.getByTestId('form-start')).toHaveAttribute('type', 'datetime-local')
  })

  test('an all-day event is written as a DATE, not a DATE-TIME', async ({ mock, page, connected }) => {
    void connected
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-all-day').check()
    await page.getByTestId('form-summary').fill('Public holiday')
    await page.getByTestId('form-start').fill(isoDay(0))
    await page.getByTestId('form-end').fill(isoDay(0))
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('event-form')).toHaveCount(0)
    const put = mock.matching('PUT')
    expect(put).toHaveLength(1)
    expect(put[0].body).toMatch(/DTSTART;VALUE=DATE:\d{8}/)
    expect(put[0].body).not.toMatch(/DTSTART:\d{8}T/)
  })

  test('the form opens pre-dated to the selected day', async ({ connected: page }) => {
    await dayCell(page, isoDay(4)).click()
    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-start')).toHaveValue(new RegExp('^' + isoDay(4)))
  })

  test('Add event with no day selected selects today', async ({ connected: page }) => {
    await expect(page.locator('[data-testid="day-cell"][data-selected="true"]')).toHaveCount(0)
    await page.getByTestId('add-event').click()
    await expect(dayCell(page, isoDay(0))).toHaveAttribute('data-selected', 'true')
    await expect(page.getByTestId('form-start')).toHaveValue(new RegExp('^' + isoDay(0)))
  })

  test('a custom reminder reveals its value and unit fields', async ({ connected: page }) => {
    await page.getByTestId('add-event').click()
    await expect(page.getByTestId('form-reminder-value')).toHaveCount(0)
    await page.getByTestId('form-reminder-preset').selectOption('custom')
    await expect(page.getByTestId('form-reminder-value')).toBeVisible()
    await expect(page.getByTestId('form-reminder-unit')).toBeVisible()
  })

  test('a reminder is written as a VALARM', async ({ mock, page, connected }) => {
    void connected
    await page.getByTestId('add-event').click()
    await page.getByTestId('form-summary').fill('With a reminder')
    await page.getByTestId('form-start').fill(localDateTime(0, 9))
    await page.getByTestId('form-end').fill(localDateTime(0, 10))
    await page.getByTestId('form-reminder-preset').selectOption('15')
    await page.getByTestId('form-submit').click()

    await expect(page.getByTestId('event-form')).toHaveCount(0)
    expect(mock.matching('PUT')[0].body).toContain('BEGIN:VALARM')
  })

  test('saving refreshes the list from the server, not from local state', async ({ mock, page, connected }) => {
    void connected
    mock.clearRequests()
    await fillNewEvent(page, { summary: 'Round trip', start: localDateTime(0, 20), end: localDateTime(0, 21) })
    await page.getByTestId('form-submit').click()
    // A REPORT after the PUT is what proves the list came back off the wire.
    // Polled rather than read once: the refresh is dispatched from inside the
    // save's future, so it starts after the form has already closed.
    await expect
      .poll(() => {
        const kinds = mock.requests.map((r) => r.method)
        return kinds.lastIndexOf('REPORT') - kinds.indexOf('PUT')
      }, { timeout: 15_000 })
      .toBeGreaterThan(0)
    expect(mock.requests.map((r) => r.method)).toContain('PUT')
  })
})
