import { SHIPS, DEFENCES } from "@/fleet/catalog";
const names: Readonly<Record<string, string>> = {
  "202": "Kleiner Transporter",
  "203": "Großer Transporter",
  "204": "Leichter Jäger",
  "205": "Schwerer Jäger",
  "206": "Kreuzer",
  "207": "Schlachtschiff",
  "208": "Kolonieschiff",
  "209": "Recycler",
  "210": "Spionagesonde",
  "211": "Bomber",
  "212": "Solarsatellit",
  "213": "Zerstörer",
  "214": "Todesstern",
  "215": "Schlachtkreuzer",
  "217": "Crawler",
  "218": "Reaper",
  "219": "Pathfinder",
  "401": "Raketenwerfer",
  "402": "Leichtes Lasergeschütz",
  "403": "Schweres Lasergeschütz",
  "404": "Gaußkanone",
  "405": "Ionengeschütz",
  "406": "Plasmawerfer",
  "407": "Kleine Schildkuppel",
  "408": "Große Schildkuppel",
};
export const unitName = (id: string) => names[id] ?? id;
export const availableTypes = (attacker: boolean) =>
  attacker ? SHIPS.filter((unit) => !["212", "217"].includes(unit.id)) : [...SHIPS, ...DEFENCES];
