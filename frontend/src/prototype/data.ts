// PROTOTYPE — throwaway. Static demo data for the #7 layout variants.
// Never imported by production code; lives and dies on prototype/7-layout.

export const SHIP_NAMES: Record<string, string> = {
  "202": "Small Cargo",
  "203": "Large Cargo",
  "204": "Light Fighter",
  "205": "Heavy Fighter",
  "206": "Cruiser",
  "207": "Battleship",
  "208": "Colony Ship",
  "209": "Recycler",
  "210": "Espionage Probe",
  "211": "Bomber",
  "212": "Solar Satellite",
  "213": "Destroyer",
  "214": "Deathstar",
  "215": "Battlecruiser",
  "217": "Crawler",
  "218": "Reaper",
  "219": "Pathfinder",
};

export const DEFENCE_NAMES: Record<string, string> = {
  "401": "Rocket Launcher",
  "402": "Light Laser",
  "403": "Heavy Laser",
  "404": "Gauss Cannon",
  "405": "Ion Cannon",
  "406": "Plasma Turret",
  "407": "Small Shield Dome",
  "408": "Large Shield Dome",
};

export interface DemoSlot {
  id: string;
  name: string;
  entities: Record<string, number>;
}

export const DEMO_ATTACKER_SLOTS: DemoSlot[] = [
  {
    id: "A1",
    name: "Main fleet",
    entities: { "206": 500, "207": 120, "215": 80, "203": 40 },
  },
  { id: "A2", name: "ACS partner", entities: { "204": 2000, "206": 150 } },
];

export const DEMO_DEFENDER_SLOTS: DemoSlot[] = [
  {
    id: "D1",
    name: "Home world",
    entities: { "204": 3000, "401": 400, "406": 25, "408": 1 },
  },
];

export const DEMO_TECH = { weapon: 14, shield: 12, armour: 13 };

export const DEMO_RESULT = {
  attackerWinRate: 0.83,
  defenderWinRate: 0.09,
  drawRate: 0.08,
  averageRounds: 3.4,
  attackerLosses: { "204": 412, "206": 31 },
  defenderLosses: { "204": 3000, "401": 400, "406": 25, "408": 1 },
  debris: { metal: 1_845_000, crystal: 1_212_000 },
  loot: { metal: 240_000, crystal: 180_000, deuterium: 60_000 },
  attackerProfit: 1_930_000,
  defenderProfit: -4_120_000,
  moonChance: 20,
  recyclersNeeded: 153,
};

export const fmt = (n: number): string => n.toLocaleString("en-US");
