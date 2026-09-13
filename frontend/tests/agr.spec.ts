import { expect, test } from "@playwright/test";

// Entirely invented handoff; no identities, positions, report keys or live data.
const fixture = {
  "0": [
    {
      ships: { "206": { count: 101 } },
      research: { "109": { level: 15 }, "110": { level: 13 }, "111": { level: 14 } },
      class: 2,
      allianceClass: 1,
      lifeformBonuses: {
        BaseStatsBooster: {
          "206": { armor: 0.125, shield: 0.125, weapon: 0.125, cargo: 0.1, speed: 0.1 },
        },
        CharacterClassBooster: { "1": 0, "2": 0, "3": 0 },
        ShipFuelConsumption: 0,
      },
    },
  ],
  "1": [{ ships: { "204": { count: 1000 } } }],
  settings: { rapid_fire: "1", def_to_tF: "0", debris_factor: "0.7", deuterium_in_debris: "1" },
};
const hash = (payload: unknown) =>
  `#prefill=${Buffer.from(JSON.stringify(payload)).toString("base64")}`;

test("AGR language launch, effective levels, lifeforms, immutable selection and explicit request", async ({
  page,
}) => {
  const requests: unknown[] = [];
  await page.route("**/api/simulate", async (route) => {
    requests.push(route.request().postDataJSON());
    await route.fulfill({ status: 500, body: "synthetic failure" });
  });
  await page.goto(`/de${hash(fixture)}`);
  const own = page.getByRole("region", { name: "Deine Flotte", exact: true });
  const quantity = own.getByLabel("Kreuzer Menge", { exact: true });
  await expect(quantity).toHaveValue("101");
  await expect(own.getByLabel("Waffen", { exact: true })).toHaveValue("12");
  await own.locator(".modifiers summary").click();
  await expect(own.getByLabel("Spielerklasse", { exact: true })).toHaveValue("general");
  await expect(own.getByLabel("Allianzklasse", { exact: true })).toHaveValue("warrior");
  await expect(own.getByLabel("Lebensform Kreuzer Waffen %", { exact: true })).toHaveValue("12.5");
  await quantity.fill("0");
  await expect(own.locator("tbody tr")).toHaveCount(1);
  await own.getByRole("button", { name: "Wiederherstellen" }).click();
  await expect(quantity).toHaveValue("101");
  await quantity.fill("50");
  expect(requests).toHaveLength(0);
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  expect(requests).toEqual([
    expect.objectContaining({
      attacker: {
        technology: { weapon: 12, shield: 10, armour: 11 },
        entities: { "206": 50 },
        lifeform: { "206": { weapon: 12.5, shield: 12.5, armour: 12.5 } },
      },
      attacker_bonuses: { player_class: "general", alliance_class: "warrior" },
      universe_settings: { debris_fleet: 70, debris_defence: 0, debris_deuterium: true },
    }),
  ]);
  await expect(quantity).toHaveValue("50");
  await page.evaluate(
    (value) => {
      location.hash = value;
    },
    hash({ "0": [{ ships: { "203": { count: 8 } } }] }),
  );
  await expect(own.getByLabel("Großer Transporter Menge", { exact: true })).toHaveValue("8");
  await expect(quantity).toHaveCount(0);
  expect(requests).toHaveLength(1);
});

for (const [name, payload] of [
  ["broken", "#prefill=broken!"],
  ["oversized", `#prefill=${"a".repeat(131073)}`],
  ["missing fleet", hash({ "0": [{ research: {} }] })],
  ["multiple participants", hash({ ...fixture, "0": [fixture["0"][0], fixture["0"][0]] })],
  ["ambiguous class", hash({ "0": [{ ships: {}, class: 2 }] })],
  ["unknown bonuses", hash({ "0": [{ ships: {}, bonuses: { weapon: 2 } }] })],
  ["negative quantity", hash({ "0": [{ ships: { "206": { count: -1 } } }] })],
] as const) {
  test(`${name} preserves existing planning`, async ({ page }) => {
    await page.goto(`/de${hash(fixture)}`);
    const quantity = page.getByLabel("Kreuzer Menge", { exact: true });
    await quantity.fill("20");
    await page.evaluate((value) => {
      location.hash = value;
    }, payload);
    await expect(page.getByRole("alert")).toBeVisible();
    await expect(quantity).toHaveValue("20");
  });
}

test("missing values remain blank and launch without prefill stays manual", async ({ page }) => {
  await page.goto("/de");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Simulieren", exact: true })).toBeDisabled();
  await page.goto(`/pt-BR${hash({ "0": [{ ships: { "206": { count: 0 } } }] })}`);
  await expect(page.getByLabel("Kreuzer Menge", { exact: true })).toHaveValue("0");
  const own = page.getByRole("region", { name: "Deine Flotte", exact: true });
  await expect(own.getByLabel("Waffen", { exact: true })).toHaveValue("");
  await expect(own.getByLabel("Waffen", { exact: true })).toHaveClass("missing");
});

test("metadata passthrough and missing defence percentage are explicit", async ({ page }) => {
  let request: unknown;
  await page.route("**/api/simulate", async (route) => {
    request = route.request().postDataJSON();
    await route.fulfill({ status: 500, body: "synthetic failure" });
  });
  await page.goto(
    `/de${hash({
      "0": [{ ships: { "206": { count: 10 } }, research: { "115": { level: 8 } } }],
      "1": [{ defence: { "401": { count: 20 } } }],
      settings: {
        def_to_tF: 1,
        debris_factor: 0.3,
        galaxies: 9,
        systems: 499,
        speed_fleet: 2,
        donut_galaxy: 1,
        donut_system: 0,
        global_deuterium_save_factor: 0.5,
      },
    })}`,
  );
  await page.locator(".universe summary").click();
  await expect(page.getByLabel("Verteidigung ins Trümmerfeld %", { exact: true })).toHaveValue("");
  await page.getByText("AGR übernommen · Quellen der Übergabe", { exact: true }).click();
  await expect(
    page.getByText("Verteidigungs-TF aktiviert, Anteil fehlt: bitte ergänzen", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Simulieren", exact: true }).click();
  await expect(page.getByRole("alert")).toBeVisible();
  expect(request).toMatchObject({
    attacker: { technology: { combustion: 8 } },
    universe_settings: {
      galaxies: 9,
      systems: 499,
      fleet_speed: 2,
      donut_galaxy: true,
      donut_systems: false,
      deuterium_save_factor: 0.5,
      debris_defence: 0,
    },
  });
});
