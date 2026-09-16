import { expect, test, type Page } from "@playwright/test";
import type { CombatRequest } from "../src/api/types";

// Invented contract examples only: no capture, identity, position or usable key.
const key = `sr-en-1-${"0".repeat(40)}`;
const prefill = {
  "0": [{ ships: { "206": { count: 101 } }, research: { "109": { level: 12 } } }],
  "1": [{ ships: { "204": { count: 1000 } } }],
  settings: { rapid_fire: "1", debris_factor: "0.7", def_to_tF: "0", deuterium_in_debris: "1" },
};
const candidate = {
  report_kind: "espionage",
  provenance: { source: "ogapi.faw-kes.de", community: "en", universe: 1, event_timestamp: null, game_version: null },
  defenders: [{
    character_class_id: null, alliance_class_id: null,
    entities: null, ships: { "204": 12 }, defenses: null,
    technology: { basis: "researched", weapon: 8, shield: null, armour: 0 },
    reported_base_stats_booster: { "204": { weapon: 2.7 } },
  }],
};
const encoded = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64");
const fragment = (value: unknown) => `#prefill=${encoded(value)}`;
const ownFleet = (page: Page) => page.getByRole("region", { name: "Deine Flotte", exact: true });
const enemyFleet = (page: Page) => page.getByRole("region", { name: "Gegner", exact: true });
const importButton = (page: Page) => page.getByRole("button", { name: "Bericht übertragen & laden" });
const simulateButton = (page: Page) => page.getByRole("button", { name: "Simulieren", exact: true });

async function changeHandoff(page: Page, value: unknown) {
  await page.evaluate((hash) => { location.hash = hash; }, fragment(value));
}

for (const live of [false, true]) {
  test(live ? "combined entry real local API smoke" : "combined entry through two explicit attack attempts", async ({ page }) => {
    test.skip(live && !process.env["LIVE_API"], "Opt in with LIVE_API=1 and combat-api on port 3000");
    const imports: unknown[] = [];
    const simulations: CombatRequest[] = [];
    await page.route("**/api/reports/import", async (route) => {
      imports.push(route.request().postDataJSON());
      await route.fulfill({ json: candidate });
    });
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/api/simulate") simulations.push(request.postDataJSON());
    });
    if (!live) await page.route("**/api/simulate", (route) => route.fulfill({ json: {
      results: {
        simulations: 100, attacker_wins: 75, defender_wins: 15, draws: 10,
        results: Array.from({ length: 100 }, () => ({
          debris_field: { metal: 300, crystal: 100, deuterium: 0 },
          loot: { metal: 0, crystal: 0, deuterium: 0 }, attacker_profit: simulations.length === 1 ? -1600 : -600,
        })),
      },
      report: { economics: { attacker_profit: simulations.length === 1 ? -1600 : -600, debris_field: { metal: 300, crystal: 100, deuterium: 0 } } },
    } }));
    await page.goto(`/de?SR_KEY=${key}${fragment(prefill)}`);
    await expect(page).not.toHaveURL(/SR_KEY/);
    const own = ownFleet(page).getByLabel("Kreuzer Menge", { exact: true });
    const enemy = enemyFleet(page).getByLabel("Leichter Jäger Menge", { exact: true });
    await expect(own).toHaveValue("101");
    await expect(enemy).toHaveValue("1000");
    expect(imports).toHaveLength(0);
    await own.fill("50");
    await importButton(page).click();
    await expect(enemy).toHaveValue("12");
    await expect(own).toHaveValue("50");
    expect(imports).toEqual([{ key, consent: true }]);
    expect(simulations).toHaveLength(0);
    await page.getByText("Gegner übernommen · Herkunft & Annahmen prüfen", { exact: true }).click();
    await expect(page.locator(".report-source")).toContainText("Dies ist kein Verified battle input");
    await expect(page.locator(".report-source")).toContainText("nicht aus diesem Bericht ermittelt");
    await expect(enemyFleet(page).getByLabel("Schilde", { exact: true })).toHaveValue("");
    await page.locator(".universe summary").click();
    await expect(page.getByLabel("Flotte ins Trümmerfeld %", { exact: true })).toHaveValue("70");
    // Manual settings are authoritative for the scenario, even for another report universe.
    await page.getByLabel("Flotte ins Trümmerfeld %", { exact: true }).fill("40");
    await page.getByLabel("Rapidfire", { exact: true }).uncheck();
    await simulateButton(page).click();
    const result = page.getByRole("region", { name: "Ergebnis", exact: true });
    await expect(result).toContainText("VERSUCH 1");
    await own.fill("25");
    await expect(result).toContainText("Veraltet");
    expect(simulations).toHaveLength(1);
    await simulateButton(page).click();
    const comparison = page.getByRole("table", { name: "Versuchsvergleich" });
    await expect(comparison).toContainText("Letzter · Versuch 2");
    await expect(comparison).toContainText("Vorheriger · Versuch 1");
    await expect(result).not.toContainText("Veraltet");
    await expect(comparison.getByRole("columnheader", { name: "Deine Verluste Ø" })).toBeVisible();
    await expect(comparison.getByRole("columnheader", { name: "Teilgewinn Ø" })).toBeVisible();
    if (!live) {
      await expect(comparison.getByRole("row", { name: /Letzter/ })).toContainText("1.000");
      await expect(comparison.getByRole("row", { name: /Letzter/ })).toContainText("-600");
      await expect(comparison.getByRole("row", { name: /Vorheriger/ })).toContainText("2.000");
      await expect(comparison.getByRole("row", { name: /Vorheriger/ })).toContainText("-1.600");
    }
    for (const [attempt, count] of [[1, 50], [2, 25]]) {
      await result.getByText(`Eingaben von Versuch ${String(attempt)}`, { exact: true }).click();
      await expect(result.locator("details").filter({ hasText: `Eingaben von Versuch ${String(attempt)}` })).toContainText(`Kreuzer: ${String(count)}`);
    }
    expect(simulations).toHaveLength(2);
    for (const [index, count] of [50, 25].entries()) expect(simulations[index]).toMatchObject({
      attacker: { entities: { "206": count }, technology: { weapon: 12 } },
      defender: { entities: { "204": 12 }, technology: { weapon: 8, shield: 0, armour: 0 }, lifeform: { "204": { weapon: 0 } } },
      universe_settings: { debris_fleet: 40, debris_defence: 0, debris_deuterium: true },
      use_rapid_fire: false,
    });
    if (live) await page.screenshot({ path: test.info().outputPath("combined-entry.png"), fullPage: true });
    await ownFleet(page).getByRole("button", { name: "Wiederherstellen" }).click();
    await expect(own).toHaveValue("101");
    expect(simulations).toHaveLength(2);
    await expect(page.getByRole("alert")).toHaveCount(0);
  });
}

for (const keyFirst of [false, true]) {
  test(`combined fragment preserves base64 and either parameter order: key first ${String(keyFirst)}`, async ({ page }) => {
    // Unicode passthrough metadata deliberately gives raw base64 a '+' character.
    const payload = { ...prefill, settings: { ...prefill.settings, server: "\u083e" } };
    expect(encoded(payload)).toContain("+");
    const parts = [`prefill=${encoded(payload)}`, `SR_KEY=${key}`];
    if (keyFirst) parts.reverse();
    await page.goto(`/de#${parts.join("&")}`);
    await expect(ownFleet(page).getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("101");
    await expect(page.getByLabel("Berichtsschlüssel (sr- oder cr-)")).toHaveValue(key);
    await expect(page).not.toHaveURL(/SR_KEY/);
    await expect(page.getByRole("alert")).toHaveCount(0);
  });
}

test("report first, then own-only AGR handoff preserves the imported opponent", async ({ page }) => {
  await page.route("**/api/reports/import", (route) => route.fulfill({ json: candidate }));
  await page.goto(`/de?SR_KEY=${key}`);
  await importButton(page).click();
  await expect(enemyFleet(page).getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("12");
  await changeHandoff(page, { "0": prefill["0"], settings: prefill.settings });
  await expect(ownFleet(page).getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("101");
  await expect(enemyFleet(page).getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("12");
});

for (const change of ["own selection", "new defender handoff", "manual defender"] as const) {
  test(`late report respects ${change}`, async ({ page }) => {
    let release: (() => void) | undefined;
    let started = false;
    let finished = false;
    await page.route("**/api/reports/import", async (route) => {
      started = true;
      await new Promise<void>((resolve) => { release = resolve; });
      await route.fulfill({ json: candidate });
      finished = true;
    });
    await page.goto(`/de?SR_KEY=${key}${fragment(prefill)}`);
    await importButton(page).click();
    await expect.poll(() => started).toBeTruthy();
    const own = ownFleet(page).getByLabel("Kreuzer Menge", { exact: true });
    const enemy = enemyFleet(page).getByLabel("Leichter Jäger Menge", { exact: true });
    const canceled = change === "own selection" ? undefined : page.waitForEvent("requestfailed", {
      predicate: (request) => new URL(request.url()).pathname === "/api/reports/import",
    });
    if (change === "new defender handoff") {
      await changeHandoff(page, { ...prefill, "1": [{ ships: { "204": { count: 33 } } }] });
      await expect(enemy).toHaveValue("33");
    } else if (change === "manual defender") await enemy.fill("44");
    await own.fill("25");
    await page.locator(".universe summary").click();
    await page.getByLabel("Flotte ins Trümmerfeld %", { exact: true }).fill("40");
    if (canceled) await canceled;
    release?.();
    await expect.poll(() => finished).toBeTruthy();
    await expect(page.getByRole("status")).toHaveCount(0);
    await expect(enemy).toHaveValue(change === "own selection" ? "12" : change === "manual defender" ? "44" : "33");
    await expect(own).toHaveValue("25");
    await expect(page.getByLabel("Flotte ins Trümmerfeld %", { exact: true })).toHaveValue("40");
  });
}
