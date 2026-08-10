// Fleet entry region — placeholder.
//
// This is one of the three layout regions the app shell establishes. It is
// deliberately an empty placeholder: actual fleet composition input (attacker
// and defender ships, counts, multi-slot ACS) is a sibling issue and slots in
// here without touching the other two regions. See issue #22 for the seam this
// exists to provide.
//
// It owns no state and no logic that belongs to the technology or results
// regions.

export function FleetEntry() {
  return (
    <section
      aria-labelledby="fleet-entry-heading"
      className="rounded-lg border border-slate-800 bg-slate-900/40 p-4"
    >
      <h2
        id="fleet-entry-heading"
        className="text-sm font-semibold uppercase tracking-wide text-slate-400"
      >
        Fleet entry
      </h2>
      <div className="mt-3 flex min-h-[8rem] items-center justify-center rounded border border-dashed border-slate-700 p-4 text-center text-sm text-slate-500">
        <p>
          Attacker and defender fleet composition goes here.
          <br />
          <span className="text-slate-600">
            Placeholder — implemented in a sibling issue.
          </span>
        </p>
      </div>
    </section>
  );
}
