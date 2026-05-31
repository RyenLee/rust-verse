import { expect, test } from '@playwright/test'

test.describe('Smoke Tests', () => {
  test('app loads and shows dashboard', async ({ page }) => {
    await page.goto('/')
    // The app should show either the loading screen, onboarding, or dashboard
    await page.waitForSelector('text=Dashboard', { timeout: 10000 })
    await expect(page.locator('h1')).toContainText('Dashboard')
  })

  test('sidebar navigation is visible', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Rust Manager', { timeout: 10000 })
    await expect(page.locator('nav')).toBeVisible()
    await expect(page.locator('text=Toolchains')).toBeVisible()
    await expect(page.locator('text=Components')).toBeVisible()
    await expect(page.locator('text=Targets')).toBeVisible()
    await expect(page.locator('text=Overrides')).toBeVisible()
    await expect(page.locator('text=Plugins')).toBeVisible()
  })

  test('navigate to toolchains page', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Toolchains', { timeout: 10000 })
    await page.click('a[href="/toolchains"]')
    await expect(page.locator('h1')).toContainText('Toolchains')
  })

  test('navigate to plugins page', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Plugins', { timeout: 10000 })
    await page.click('a[href="/plugins"]')
    await expect(page.locator('h1')).toContainText('Cargo Plugins')
  })

  test('navigate to components page', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Components', { timeout: 10000 })
    await page.click('a[href="/components"]')
    await expect(page.locator('h1')).toContainText('Components')
  })

  test('navigate to targets page', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Targets', { timeout: 10000 })
    await page.click('a[href="/targets"]')
    await expect(page.locator('h1')).toContainText('Targets')
  })

  test('navigate to overrides page', async ({ page }) => {
    await page.goto('/')
    await page.waitForSelector('text=Overrides', { timeout: 10000 })
    await page.click('a[href="/overrides"]')
    await expect(page.locator('h1')).toContainText('Directory Overrides')
  })
})
