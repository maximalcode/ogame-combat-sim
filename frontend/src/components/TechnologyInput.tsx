// Technology input region — placeholder.
//
// One of the three layout regions. Empty placeholder for technology levels,
// player classes, alliance classes and lifeform research — all of which feed
// `PartyData` and `PlayerBonuses` on the request. Sibling issue, slots in
// here. Owns no fleet or results logic.

export function TechnologyInput() {
  return (
    <section
      aria-labelledby="technology-input-heading"
      className="rounded-lg border border-slate-800 bg-slate-900/40 p-4"
    >
      <h2
        id="technology-input-heading"
        className="text-sm font-semibold uppercase tracking-wide text-slate-400"
      >
        Technology &amp; bonuses
      </h2>
      <div className="mt-3 flex min-h-[8rem] items-center justify-center rounded border border-dashed border-slate-700 p-4 text-center text-sm text-slate-500">
        <p>
          Weapons / shielding / armour levels, classes and lifeform research go
          here.
          <br />
          <span className="text-slate-600">
            Placeholder — implemented in a sibling issue.
          </span>
        </p>
      </div>
    </section>
  );
}
