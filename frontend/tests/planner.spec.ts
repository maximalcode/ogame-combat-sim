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

test('manual snapshot corrections and defender quantities stay editable', async ({ page }) => {
  const { own, enemy } = await prepare(page);
  await own.locator('.unit-picker summary').click();
  await own.getByLabel('Bestand Kreuzer', { exact: true }).fill('40');
  await expect(own.getByLabel('Kreuzer Menge', { exact: true })).toHaveValue('40');
  await own.getByLabel('Bestand Kreuzer', { exact: true }).fill('80');
  await expect(own.getByLabel('Kreuzer Menge', { exact: true })).toHaveValue('40');
  await own.getByRole('button', { name: 'Wiederherstellen' }).click();
  await expect(own.getByLabel('Kreuzer Menge', { exact: true })).toHaveValue('80');
  await enemy.getByLabel('Leichter Jäger Menge', { exact: true }).fill('2000');
  await enemy.getByRole('button', { name: 'Leichter Jäger weniger' }).click();
  await expect(enemy.getByLabel('Leichter Jäger Menge', { exact: true })).toHaveValue('1999');
});
