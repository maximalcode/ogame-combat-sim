# Keep combat modes behind one request and report boundary

Fleet battles and missile attacks will use an explicit combat mode within the
shared request and report boundary. Missile resolution remains a separate
domain path with direct defence damage, interception, and no combat rounds,
while API, CLI, and UI keep one vocabulary for submitting and reporting combat.
