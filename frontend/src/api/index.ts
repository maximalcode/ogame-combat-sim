// Barrel for the API layer. Call sites import from `@/api` rather than
// reaching into individual files.
export { ApiError, postSimulate, API_ROUTES } from "@/api/client";
export type {
  CombatRequest,
  SimulationResponse,
  CombatResults,
  CombatReport,
  CombatOutcome,
  BattleType,
  PartyData,
  Technology,
  FleetComposition,
  EntityType,
} from "@/api/types";
