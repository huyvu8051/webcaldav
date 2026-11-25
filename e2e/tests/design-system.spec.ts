import { test, expect, seedSession, waitForIdle, isoDay, dayCell } from '../fixtures/app'

/**
 * DESIGN.md is vendored into this repo and these are its non-negotiables.
 * They are asserted on computed styles rather than on class names, because a
 * class list says what was intended and a computed style says what shipped.
 *
 * Real Tailwind is loaded for this spec -- the tokens these rules are about
 * only exist once it has run.
 */
// Opting the whole file into the real Tailwind build. The fixture installs
// the mock, so this has to be a test option rather than a call on the mock.
test.use({ realTailwind: true })

test.describe('the design system holds', () => {
  test.beforeEach(async ({ mock, page }) => {
    void mock
    await seedSession(page)
  })

  test('nothing anywhere has a shadow', async ({ page }) => {
    // "Depth comes only from surface colour change and 1px hairline borders."
    for (const url of ['/', '/calendar']) {
      await page.goto(url)
      await page.waitForTimeout(1200)
      const shadowed = await page.evaluate(() =>
        [...document.querySelectorAll('*')]
          .filter((e) => {
            const s = getComputedStyle(e).boxShadow
            return s && s !== 'none'
          })
          .map((e) => `${e.tagName.toLowerCase()}.${(e.className || '').toString().slice(0, 60)}`),
      )
      expect(shadowed, `box-shadow found on ${url}`).toEqual([])
    }
  })

  test('nothing anywhere has a gradient', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    const gradients = await page.evaluate(() =>
      [...document.querySelectorAll('*')]
        .filter((e) => getComputedStyle(e).backgroundImage.includes('gradient'))
        .map((e) => e.tagName.toLowerCase()),
    )
    expect(gradients).toEqual([])
  })

  test('no element uses font-weight 500', async ({ page }) => {
    // The ladder is 300/400/600/700. 500 is absent from the system and
    // reaching for it is the usual tell that a component was eyeballed.
    await page.goto('/calendar')
    await waitForIdle(page)
    const fivehundreds = await page.evaluate(() =>
      [...document.querySelectorAll('*')]
        .filter((e) => getComputedStyle(e).fontWeight === '500')
        .map((e) => e.tagName.toLowerCase()),
    )
    expect(fivehundreds).toEqual([])
  })

  test('body text is 17px, not 16', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    const size = await page.evaluate(() => getComputedStyle(document.body).fontSize)
    expect(size).toBe('17px')
  })

  test('every tap target is at least 44px tall', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    await dayCell(page, isoDay(0)).click()
    await page.getByTestId('add-event').click()

    // A control inside a >=44px <label> is tappable across the whole row, so
    // the row is the target and the box is just the glyph. Measuring the
    // input alone would flag a checkbox that is perfectly reachable.
    const small = await page.evaluate(() => {
      const effective = (e: Element): number => {
        const label = e.closest('label')
        return Math.max(e.getBoundingClientRect().height, label?.getBoundingClientRect().height ?? 0)
      }
      return [...document.querySelectorAll('button, select, input')]
        .filter((e) => {
          const r = e.getBoundingClientRect()
          return r.width > 0 && r.height > 0 && effective(e) < 43.5
        })
        .map((e) => `${e.tagName.toLowerCase()}[${e.getAttribute('data-testid') ?? e.getAttribute('aria-label') ?? e.getAttribute('type') ?? ''}] h=${Math.round(effective(e))}`)
    })
    expect(small).toEqual([])
  })

  test('Disconnect is a full-height target, not a 24px one', async ({ page }) => {
    // It was 24px: `px-1 py-0.5` on bare text, sitting at the screen edge in
    // a 53px bar. Found by the tap-target sweep above.
    await page.goto('/calendar')
    await waitForIdle(page)
    const box = await page.getByTestId('disconnect').boundingBox()
    expect(box!.height).toBeGreaterThanOrEqual(43.5)
  })

  test('interactive colour is one blue and nothing else', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    // Sampled only once nothing is in flight: a disabled button is composited
    // toward its background, and the blend (measured at 18,105,192) is not a
    // second accent, it is the same one being greyed.
    await expect(page.getByTestId('refresh')).toBeEnabled()

    const accents = await page.evaluate(() => {
      const seen = new Set<string>()
      for (const e of document.querySelectorAll('button, a[role="button"]')) {
        const s = getComputedStyle(e)
        for (const c of [s.color, s.backgroundColor, s.borderTopColor]) {
          const m = c.match(/^rgba?\((\d+), (\d+), (\d+)/)
          if (!m) continue
          const [r, g, b] = [+m[1], +m[2], +m[3]]
          const grey = Math.abs(r - g) < 12 && Math.abs(g - b) < 12
          if (!grey) seen.add(`${r},${g},${b}`)
        }
      }
      return [...seen]
    })

    // Hue, not RGB distance. The rule is "one accent", and a hue angle is
    // what that actually means: every browser composites a disabled button's
    // colour differently (WebKit gave 18,105,192, Chrome 36,108,180, both of
    // them Action Blue being greyed), and chasing those with a distance
    // threshold is chasing renderers. A red delete is 0 degrees, a green
    // success 120, an amber warning 40 -- none of them land near 210.
    const hue = (r: number, g: number, b: number): number => {
      const [R, G, B] = [r / 255, g / 255, b / 255]
      const max = Math.max(R, G, B)
      const min = Math.min(R, G, B)
      if (max === min) return NaN
      const d = max - min
      const h = max === R ? ((G - B) / d) % 6 : max === G ? (B - R) / d + 2 : (R - G) / d + 4
      return (h * 60 + 360) % 360
    }
    const ACTION_BLUE_HUE = hue(0, 0x66, 0xcc)

    expect(accents.length).toBeGreaterThan(0)
    for (const a of accents) {
      const [r, g, b] = a.split(',').map(Number)
      const h = hue(r, g, b)
      expect(Math.abs(h - ACTION_BLUE_HUE), `rgb(${a}) is hue ${Math.round(h)}, not Action Blue's ${ACTION_BLUE_HUE}`).toBeLessThan(10)
    }
  })

  test('every select draws its chevron at the same inset', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    await page.getByTestId('add-event').click()
    await page.getByTestId('month-title').click()

    // The browser's own arrow sits hard against the right edge -- outside the
    // pill's curve, and at a different place for every control width. One
    // chevron, one inset, is the fix.
    const insets = await page.evaluate(() =>
      [...document.querySelectorAll('select')].map((e) => {
        const s = getComputedStyle(e)
        return `${s.appearance}|${s.backgroundPosition}|${s.paddingRight}`
      }),
    )
    expect(insets.length).toBeGreaterThan(3)
    expect(new Set(insets).size, `selects disagree: ${JSON.stringify([...new Set(insets)])}`).toBe(1)
    expect(insets[0]).toContain('none')
  })

  test('the radius ladder is 8 / 11 / 18 / pill and nothing between', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    const radii = await page.evaluate(() => {
      const ok = new Set<string>()
      for (const e of document.querySelectorAll('*')) {
        const r = getComputedStyle(e).borderTopLeftRadius
        if (r && r !== '0px') ok.add(r)
      }
      return [...ok]
    })
    const allowed = new Set(['8px', '11px', '18px', '9999px', '50%'])
    for (const r of radii) {
      if (r.endsWith('%')) continue
      expect(allowed.has(r), `radius ${r} is not on the ladder`).toBe(true)
    }
  })

  test('the type stack is the system font, with no webfont for text', async ({ page }) => {
    await page.goto('/')
    await page.waitForTimeout(1000)
    const family = await page.evaluate(() => getComputedStyle(document.body).fontFamily)
    expect(family).toMatch(/-apple-system|system-ui|BlinkMacSystemFont/)
  })

  test('nothing is set in all caps or wide tracking', async ({ page }) => {
    await page.goto('/calendar')
    await waitForIdle(page)
    const shouty = await page.evaluate(() =>
      [...document.querySelectorAll('*')]
        .filter((e) => {
          const s = getComputedStyle(e)
          const spaced = parseFloat(s.letterSpacing)
          return s.textTransform === 'uppercase' || (!Number.isNaN(spaced) && spaced > 0.5)
        })
        .map((e) => e.tagName.toLowerCase()),
    )
    expect(shouty).toEqual([])
  })
})
