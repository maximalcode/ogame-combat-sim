# Sequential attack waves

`combat_types::WaveSequenceRequest` describes ordered missions against one
single-party target. Each `AttackWave` supplies a fresh `PartyData` attacker
and optional player bonuses. The target has its own bonuses; lifeform modifiers
travel with each party. Supply resolved `DebrisSettings` (for an existing
`CombatRequest`, use its `debris_settings()` method).

Call `Combat::simulate_waves(&request, &mut rng)` for one seeded trajectory.
It consumes the same random stream as successive `Combat::simulate_single`
calls. As with that entry point, reproducibility assumes the same input fleet
map ordering, not just the same seed after independently deserializing maps.
Call `Simulator::simulate_wave_sequences(&request, count)` for independent
parallel trajectories. Every trajectory keeps its own exact survivor counts;
no averaged or fractional fleet is fed into the next wave.

The result contains input-ordered `waves`, additive `totals`, `final_defender`,
and `remaining_resources`. Every wave is a full `SimulationResult`, including
outcome, rounds, losses, surviving compositions, debris and loot. The final
defender retains researched technology and lifeform modifiers. Class bonuses
are resolved once per battle, never accumulated between waves.

Only surviving defender ships and defences carry forward. Hulls and shields
start fresh for the next mission. Attacker survivors leave; include them
explicitly in another wave if they should return. Loot is subtracted before
the next mission. Debris and losses sum newly destroyed units, not successive
target snapshots. Attacker and defender profit remain alternative assumptions
about who harvests the debris; do not add them together.

The sequence reuses the current single-battle economic calculation, including
its default 50% plunder and cargo calculation. That path currently calculates
loot from surviving attacker cargo even for a draw; wave sequencing does not
change that existing limitation. No resource production occurs between waves.

Each battle still has at most six rounds. Draws and empty targets do not stop
later missions, and there is no wave-count cap. An empty sequence returns the
unchanged target with zero totals; zero Monte Carlo runs returns an empty list.

This API runs full-size battles without downscaling. It adds no HTTP route,
CLI command, frontend, ACS slots, scheduling, or defence rebuild between waves.
