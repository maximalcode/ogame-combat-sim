import { expect, test, type Page, type Locator } from "@playwright/test";

// Synthetic HTTP response. Deliberately has fractional average ship losses:
// the UI must not value the summary report's truncated fleet composition.
const response = {
  results: {
    simulations: 100,
    attacker_wins: 75,
    defender_wins: 15,
    draws: 10,
    results: Array.from({ length: 100 }, (_, i) => ({
      debris_field: { metal: 300, crystal: 100, deuterium: 0 },
      loot: { metal: 0, crystal: 0, deuterium: 0 },
      attacker_profit: i < 50 ? -3600 : 400,
    })),
  },
  report: {
    economics: {
      attacker_profit: -1600,
      attacker_losses_cost: { metal: 0, crystal: 0, deuterium: 0 },
      debris_field: { metal: 300, crystal: 100, deuterium: 0 },
    },
  },
};
async function add(side: Locator, id: string, count: string) {
  await side.locator(".unit-picker summary").click();
  await side.getByLabel("Einheit", { exact: true }).selectOption(id);
  await side.getByLabel("Anzahl hinzufügen", { exact: true }).fill(count);
  await side.getByRole("button", { name: "Hinzufügen", exact: true }).click();
  await side.locator(".unit-picker summary").click();
}
async function prepare(page: Page) {
  await page.goto("/");
  const own = page.getByRole("region", { name: "Deine Flotte", exact: true });
  const enemy = page.getByRole("region", { name: "Gegner", exact: true });
  await add(own, "206", "101");
  await add(enemy, "204", "1000");
  return { own, enemy };
}

test("snapshot limits, rounding, blanks, restore and explicit HTTP start", async ({ page }) => {
  const requests: unknown[] = [];
  await page.route("**/api/simulate", async (route) => {
    requests.push(route.request().postDataJSON());
    await route.fulfill({ json: response });
  });
  const { own, enemy } = await prepare(page);
  const quantity = own.getByLabel("Kreuzer Menge", { exact: true });
  await expect(own.locator("tbody tr")).toHaveCount(1);
  await expect(page.getByText("LAYOUT-STUDIE")).toHaveCount(0);
  await quantity.fill("999");
  await expect(quantity).toHaveValue("101");
  await quantity.fill("-3");
  await expect(quantity).toHaveValue("0");
  await quantity.fill("9.8");
  await expect(quantity).toHaveValue("9");
  await own.getByRole("button", { name: "Kreuzer weniger" }).click();
  await expect(quantity).toHaveValue("8");
  await own.getByRole("button", { name: "Kreuzer mehr" }).click();
  await expect(quantity).toHaveValue("9");
  await own.getByRole("button", { name: "Wiederherstellen" }).click();
  await own.getByRole("button", { name: "½ Menge" }).click();
  await expect(quantity).toHaveValue("50");
  await own.getByRole("button", { name: "+10 %" }).click();
  await expect(quantity).toHaveValue("55");
  await quantity.fill("100");
  await own.getByRole("button", { name: "+10 %" }).click();
  await expect(quantity).toHaveValue("101");
  await quantity.fill("0");
  await expect(own.locator("tbody tr")).toHaveCount(1);
  await own.getByLabel("Waffen", { exact: true }).fill("12");
  await own.getByRole("button", { name: "Wiederherstellen" }).click();
  await expect(quantity).toHaveValue("101");
  await expect(own.getByLabel("Waffen", { exact: true })).toHaveValue("12");
  await expect(own.locator("tbody tr td").first()).toHaveText("101");
  await expect(own.getByLabel("Schilde", { exact: true })).toHaveValue("");
  await expect(own.getByLabel("Schilde", { exact: true })).toHaveClass("missing");
  await own.locator(".modifiers summary").click();
  await own.getByLabel("Spielerklasse", { exact: true }).selectOption("general");
  await own.getByLabel("Allianzklasse", { exact: true }).selectOption("warrior");
  await own.getByLabel("Lebensform Kreuzer Waffen %", { exact: true }).fill("12.5");
  await enemy.getByLabel("Panzerung", { exact: true }).fill("8");
  await page.locator(".universe summary").click();
  await page.getByLabel("Flotte ins Trümmerfeld %", { exact: true }).fill("70");
  await page.getByLabel("Verteidigung ins Trümmerfeld %", { exact: true }).fill("30");
  await page.getByLabel("Deuterium im TF").check();
  await page.getByLabel("Rapidfire").uncheck();
  await page.getByRole("button", { name: "1.000", exact: true }).click();
  expect(requests).toHaveLength(0);
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByText("75 %", { exact: true })).toBeVisible();
  expect(requests).toHaveLength(1);
  expect(requests[0]).toMatchObject({
    attacker: {
      entities: { "206": 101 },
      technology: { weapon: 12, shield: 0, armour: 0 },
      lifeform: { "206": { weapon: 12.5, shield: 0, armour: 0 } },
    },
    defender: { technology: { armour: 8 } },
    attacker_bonuses: { player_class: "general", alliance_class: "warrior" },
    universe_settings: { debris_fleet: 70, debris_defence: 30, debris_deuterium: true },
    simulations: 1000,
    use_rapid_fire: false,
  });
  await expect(page.getByText("2.000", { exact: true })).toBeVisible();
  await expect(own.getByLabel("Schilde", { exact: true })).toHaveValue("");
});

test("loading prevents duplicate starts; errors retain inputs and last success", async ({
  page,
}) => {
  let count = 0;
  let release: (() => void) | undefined;
  await page.route("**/api/simulate", async (route) => {
    count++;
    if (count === 1) {
      await route.fulfill({ json: response });
      return;
    }
    await new Promise<void>((resolve) => {
      release = resolve;
    });
    await route.fulfill({ status: 503, body: "Busy" });
  });
  const { own } = await prepare(page);
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByText("75 %", { exact: true })).toBeVisible();
  await own.getByLabel("Kreuzer Menge", { exact: true }).fill("30");
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByRole("button", { name: "Wird berechnet…" })).toBeDisabled();
  await expect.poll(() => count).toBe(2);
  release?.();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.getByText("75 %", { exact: true })).toBeVisible();
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("30");
});

test("blank quantities stay visible and zero selection can be restored", async ({ page }) => {
  await page.goto("/");
  const own = page.getByRole("region", { name: "Deine Flotte", exact: true });
  await add(own, "206", "");
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("");
  await expect(own.locator("tbody")).toContainText("Assumed zero");
  await expect(own.getByRole("button", { name: "Kreuzer weniger" })).toBeDisabled();
  await expect(own.getByRole("button", { name: "Kreuzer mehr" })).toBeDisabled();
  await own.getByRole("button", { name: "½ Menge" }).click();
  await own.getByRole("button", { name: "+10 %" }).click();
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("");
  await expect(page.getByRole("button", { name: "Simulieren", exact: true })).toBeDisabled();
});

for (const width of [1440, 1024]) {
  test(`desktop ${width}: keyboard and blocked images remain usable`, async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await page.route("**/*.svg", (route) => route.abort());
    const { own } = await prepare(page);
    await own.getByLabel("Kreuzer Menge", { exact: true }).focus();
    await page.keyboard.press("Tab");
    await expect(own.getByRole("button", { name: "Kreuzer mehr" })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("101");
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBeTruthy();
    await expect(own.getByRole("rowheader", { name: /Kreuzer/ })).toBeVisible();
    await page.screenshot({ path: `test-results/dock-${width}-no-images.png`, fullPage: true });
  });
}

test("real local API smoke", async ({ page }) => {
  test.skip(!process.env["LIVE_API"], "Run with LIVE_API=1 and combat-api on port 3000");
  await prepare(page);
  const responsePromise = page.waitForResponse("**/api/simulate");
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  const actual = await responsePromise;
  expect(actual.ok()).toBeTruthy();
  await expect(page.getByText("100 Durchläufe", { exact: true })).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("manual snapshot corrections and defender quantities stay editable", async ({ page }) => {
  const { own, enemy } = await prepare(page);
  await own.locator(".unit-picker summary").click();
  await own.getByLabel("Bestand Kreuzer", { exact: true }).fill("40");
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("40");
  await own.getByLabel("Bestand Kreuzer", { exact: true }).fill("80");
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("40");
  await own.getByRole("button", { name: "Wiederherstellen" }).click();
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("80");
  await enemy.getByLabel("Leichter Jäger Menge", { exact: true }).fill("2000");
  await enemy.getByRole("button", { name: "Leichter Jäger weniger" }).click();
  await expect(enemy.getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("1999");
});

test("successful attempts survive errors and a late response stays tied to its launch snapshot", async ({ page }) => {
  test.slow(); // Six HTTP attempts plus both desktop layouts.
  let release: (() => void) | undefined;
  const requests: Record<string, unknown>[] = [];
  await page.route("**/api/simulate", async (route) => {
    requests.push(route.request().postDataJSON());
    if (requests.length === 2 || requests.length === 5) {
      await route.fulfill({ status: 503, body: "Busy" });
      return;
    }
    if (requests.length === 6) {
      await route.fulfill({ json: { results: {}, report: {} } });
      return;
    }
    if (requests.length === 3) {
      await new Promise<void>((resolve) => { release = resolve; });
    }
    await route.fulfill({ json: response });
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  const { own } = await prepare(page);
  const result = page.getByRole("region", { name: "Ergebnis", exact: true });
  const start = page.getByRole("button", { name: "Simulieren", exact: true });
  await start.click();
  await expect(result).toContainText("VERSUCH 1");
  await own.getByLabel("Kreuzer Menge", { exact: true }).fill("30");
  await expect(result).toContainText("Veraltet");
  expect(requests).toHaveLength(1);
  await start.click();
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(result).toContainText("VERSUCH 1");
  await expect(page.getByRole("table", { name: "Versuchsvergleich" })).toHaveCount(0);
  await start.click();
  await expect(page.getByRole("button", { name: "Wird berechnet…" })).toBeDisabled();
  await expect.poll(() => requests.length).toBe(3);
  await own.getByLabel("Kreuzer Menge", { exact: true }).fill("20");
  await page.getByRole("button", { name: "1.000", exact: true }).click();
  release?.();
  await expect(result).toContainText("VERSUCH 3");
  await expect(result).toContainText("Veraltet");
  const comparison = page.getByRole("table", { name: "Versuchsvergleich" });
  await expect(comparison).toContainText("Letzter · Versuch 3");
  await expect(comparison).toContainText("Vorheriger · Versuch 1");
  await expect(comparison).not.toContainText("Versuch 2");
  await result.getByText("Eingaben von Versuch 3", { exact: true }).click();
  const inputs = result.locator("details").filter({ has: page.getByText("Eingaben von Versuch 3", { exact: true }) });
  await expect(inputs).toContainText("Kreuzer: 30");
  await expect(inputs).toContainText("100 Durchläufe angefordert");
  expect(requests[2]).toMatchObject({ attacker: { entities: { "206": 30 } }, simulations: 100 });
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("20");
  expect(requests).toHaveLength(3);
  await start.click();
  await expect(result).toContainText("Passend zu den aktuellen Eingaben");
  await expect(comparison).toContainText("Letzter · Versuch 4");
  await expect(comparison).toContainText("Vorheriger · Versuch 3");
  await expect(comparison).not.toContainText("Versuch 1");
  await page.screenshot({ path: "test-results/attempt-comparison-1440.png", fullPage: true });
  await page.setViewportSize({ width: 1024, height: 1000 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBeTruthy();
  await page.screenshot({ path: "test-results/attempt-comparison-1024.png", fullPage: true });
  for (let failure = 0; failure < 2; failure++) {
    await start.click();
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(comparison).toContainText("Letzter · Versuch 4");
    await expect(comparison).toContainText("Vorheriger · Versuch 3");
    await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("20");
  }
});

test("fractional resource averages and signed profit come from battles, not truncated summaries", async ({ page }) => {
  let count = 0;
  const zero = { metal: 0, crystal: 0, deuterium: 0 };
  await page.route("**/api/simulate", async (route) => {
    count++;
    await route.fulfill({ json: {
      results: {
        simulations: 3, attacker_wins: 1, draws: 1, defender_wins: 1,
        results: [
          { debris_field: { metal: 900, crystal: 300, deuterium: 0 }, loot: zero, attacker_profit: count === 1 ? -2800 : 1200 },
          { debris_field: zero, loot: zero, attacker_profit: 0 },
          { debris_field: zero, loot: zero, attacker_profit: 0 },
        ],
      },
      report: { economics: { attacker_losses_cost: zero, attacker_profit: 0, debris_field: zero } },
    } });
  });
  await prepare(page);
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  const result = page.getByRole("region", { name: "Ergebnis", exact: true });
  await expect(result.getByText("1.333,33", { exact: true })).toBeVisible();
  await expect(result.getByText("-933,33", { exact: true })).toBeVisible();
  await expect(result).toContainText("33,33 %");
  await expect(result).toContainText("1 Siege · 1 Unentschieden · 1 Niederlagen");
  await expect(result).toContainText("Trümmerfeld Ø: 300 Metall · 100 Kristall · 0 Deuterium");
  await expect(result).toContainText("Ohne Beute, Treibstoff und Wiederaufbau");
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  const comparison = page.getByRole("table", { name: "Versuchsvergleich" });
  await expect(comparison.getByRole("row", { name: /Letzter/ })).toContainText("+400");
  await expect(comparison.getByRole("row", { name: /Vorheriger/ })).toContainText("-933,33");
});

test("combat settings stale a result without automatically starting another request", async ({ page }) => {
  let count = 0;
  await page.route("**/api/simulate", async (route) => { count++; await route.fulfill({ json: response }); });
  const { own, enemy } = await prepare(page);
  await own.locator(".modifiers summary").click();
  await enemy.locator(".modifiers summary").click();
  await page.locator(".universe summary").click();
  const result = page.getByRole("region", { name: "Ergebnis", exact: true });
  const changes = [
    () => own.getByLabel("Waffen", { exact: true }).fill("1"),
    () => enemy.getByLabel("Schilde", { exact: true }).fill("2"),
    () => own.getByLabel("Spielerklasse", { exact: true }).selectOption("general"),
    () => enemy.getByLabel("Allianzklasse", { exact: true }).selectOption("warrior"),
    () => own.getByLabel("Lebensform Kreuzer Waffen %", { exact: true }).fill("12.5"),
    () => page.getByLabel("Flotte ins Trümmerfeld %", { exact: true }).fill("70"),
    () => page.getByLabel("Verteidigung ins Trümmerfeld %", { exact: true }).fill("30"),
    () => page.getByLabel("Deuterium im TF").check(),
    () => page.getByLabel("Rapidfire").uncheck(),
    () => page.getByRole("button", { name: "1.000", exact: true }).click(),
  ];
  for (const change of changes) {
    await page.getByRole("button", { name: "Simulieren", exact: true }).click();
    await expect(result).toContainText("Passend zu den aktuellen Eingaben");
    const before = count;
    await change();
    await expect(result).toContainText("Veraltet");
    expect(count).toBe(before);
  }
});
