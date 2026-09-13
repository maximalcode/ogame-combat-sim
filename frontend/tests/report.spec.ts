import { expect, test } from "@playwright/test";
import type { CombatRequest } from "../src/api/types";
const key = "sr-en-1-0000000000000000000000000000000000000000";
const candidate = {
  report_kind: "espionage",
  provenance: {
    source: "ogapi.faw-kes.de",
    community: "en",
    universe: 1,
    event_timestamp: null,
    game_version: null,
  },
  defenders: [
    {
      character_class_id: 2,
      alliance_class_id: 2,
      entities: null,
      ships: { "204": 12 },
      defenses: null,
      technology: { basis: "researched", weapon: 8, shield: null, armour: 0 },
      reported_base_stats_booster: { "204": { weapon: 2.7 } },
    },
  ],
};
test("SR_KEY requires transfer action, preserves own fleet, exposes unknowns and explicitly simulates", async ({
  page,
}) => {
  const imports: unknown[] = [];
  const simulations: CombatRequest[] = [];
  await page.route("**/api/reports/import", async (route) => {
    imports.push(route.request().postDataJSON());
    await route.fulfill({ json: candidate });
  });
  await page.route("**/api/simulate", async (route) => {
    simulations.push(route.request().postDataJSON());
    await route.fulfill({
      json: {
        results: { simulations: 100, attacker_wins: 75, defender_wins: 15, draws: 10, results: [] },
        report: {
          economics: { attacker_profit: 0, debris_field: { metal: 0, crystal: 0, deuterium: 0 } },
        },
      },
    });
  });
  await page.goto(`/?SR_KEY=${key}`);
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByLabel("Berichtsschlüssel (sr- oder cr-)")).toHaveValue(key);
  expect(imports).toHaveLength(0);
  const own = page.getByRole("region", { name: "Deine Flotte", exact: true });
  await own.locator(".unit-picker summary").click();
  await own.getByLabel("Einheit", { exact: true }).selectOption("206");
  await own.getByLabel("Anzahl hinzufügen", { exact: true }).fill("15");
  await own.getByRole("button", { name: "Hinzufügen", exact: true }).click();
  await page.getByRole("button", { name: "Bericht übertragen & laden" }).click();
  const enemy = page.getByRole("region", { name: "Gegner", exact: true });
  await expect(enemy.getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("12");
  await expect(enemy.getByLabel("Schilde", { exact: true })).toHaveValue("");
  await expect(enemy.getByLabel("Panzerung", { exact: true })).toHaveValue("0");
  await expect(own.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("15");
  await expect(page.getByLabel("Berichtsschlüssel (sr- oder cr-)")).toHaveValue("");
  expect(imports).toEqual([{ key, consent: true }]);
  expect(simulations).toHaveLength(0);
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByRole("region", { name: "Ergebnis", exact: true })).toContainText("75 %");
  expect(simulations[0].defender.technology).toMatchObject({ weapon: 8, shield: 0, armour: 0 });
  expect(simulations[0].defender_bonuses).toEqual({
    player_class: "general",
    alliance_class: "warrior",
  });
  expect(simulations[0].defender.lifeform["204"].weapon).toBe(0);
  await page.route("**/api/reports/import", (route) => route.fulfill({ status: 502 }));
  await page.getByLabel("Berichtsschlüssel (sr- oder cr-)").fill(key);
  await page.getByRole("button", { name: "Bericht übertragen & laden" }).click();
  await expect(page.getByRole("alert")).toContainText("abgelaufen");
  await expect(page.getByRole("region", { name: "Ergebnis", exact: true })).toContainText("75 %");
  await expect(enemy.getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("12");
});
test("manual import errors and older responses preserve the latest opponent", async ({ page }) => {
  let release: (() => void) | undefined;
  let calls = 0;
  await page.route("**/api/reports/import", async (route) => {
    const call = ++calls;
    if (call === 1)
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    if (call === 3) return route.fulfill({ status: 429 });
    await route.fulfill({
      json: {
        ...candidate,
        defenders: [{ ...candidate.defenders[0], ships: { "204": call === 1 ? 1 : 22 } }],
      },
    });
  });
  await page.goto("/");
  const input = page.getByLabel("Berichtsschlüssel (sr- oder cr-)");
  const submit = page.getByRole("button", { name: "Bericht übertragen & laden" });
  await input.fill(key);
  await submit.click();
  await expect(page.getByRole("status")).toBeVisible();
  await input.fill(key.replace("sr-en", "sr-de"));
  await submit.click();
  const quantity = page.getByLabel("Leichter Jäger Menge", { exact: true });
  await expect(quantity).toHaveValue("22");
  release?.();
  await input.fill(key);
  await submit.click();
  await expect(page.getByRole("alert")).toContainText("Quote");
  await expect(quantity).toHaveValue("22");
});

test("combat bonuses remain evidence and malformed responses preserve the opponent", async ({
  page,
}) => {
  await page.route("**/api/reports/import", (route) =>
    route.fulfill({
      json: {
        ...candidate,
        report_kind: "combat",
        defenders: [
          {
            ...candidate.defenders[0],
            entities: { "204": 4 },
            technology: {
              basis: "reported_combat_bonus_divided_by_ten",
              weapon: 20,
              shield: 20,
              armour: 20,
            },
          },
        ],
      },
    }),
  );
  await page.goto("/");
  const input = page.getByLabel("Berichtsschlüssel (sr- oder cr-)");
  const submit = page.getByRole("button", { name: "Bericht übertragen & laden" });
  await input.fill(key.replace("sr-", "cr-"));
  await submit.click();
  const enemy = page.getByRole("region", { name: "Gegner", exact: true });
  await expect(enemy.getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("4");
  await expect(enemy.getByLabel("Waffen", { exact: true })).toHaveValue("");
  await page.getByText("Gegner übernommen · Herkunft & Annahmen prüfen", { exact: true }).click();
  await expect(
    page.getByText("sie wurden nicht aus diesem Bericht ermittelt.", { exact: false }),
  ).toBeVisible();
  await page.route("**/api/reports/import", (route) =>
    route.fulfill({ body: "private-provider-text", contentType: "application/json" }),
  );
  await input.fill(key);
  await submit.click();
  await expect(page.getByRole("alert")).toContainText("Ungültige Importantwort");
  await expect(page.getByText("private-provider-text", { exact: false })).toHaveCount(0);
  await expect(enemy.getByLabel("Leichter Jäger Menge", { exact: true })).toHaveValue("4");
});
