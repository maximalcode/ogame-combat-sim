# Model tactical retreat from sourced game rules

Tactical retreat is an automatic pre-combat outcome for an eligible defending
fleet when the configured force ratio is exceeded. Callers provide the
threshold or disablement configured by the player and universe; they do not
select a retreat outcome directly. The defender score cutoff is universe data,
with 500,000 points as the current-universe default and 50,000 as a legacy
value, so it must not be a single engine constant. No combat means no combat
losses, debris, or loot from that encounter.
