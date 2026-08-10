// Typed client for combat-api's simulate route.
//
// The route returns a typed value rather than `any`, so call sites never reach
// into untyped JSON. Errors — network failures, non-2xx responses, malformed
// bodies — surface as a single `ApiError` type rather than an unhandled
// rejection, so the UI can render them as a visible state.

import { apiUrl } from "@/config";
import type { CombatRequest, SimulationResponse } from "@/api/types";

/** The simulate route. The API also exposes a health route at `/`, but nothing
 *  in this issue calls it — added when something needs it. */
export const API_ROUTES = {
  simulate: "/api/simulate",
} as const;

/** A surfaced error: a single shape for every failure mode the UI might see. */
export class ApiError extends Error {
  /** HTTP status, or 0 for a network/parse failure with no response. */
  readonly status: number;
  /** The endpoint that failed, for context in the UI. */
  readonly endpoint: string;

  constructor(message: string, status: number, endpoint: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.endpoint = endpoint;
  }
}

/**
 * Run a simulation. The response is typed end to end.
 *
 * Every failure mode — network error, non-2xx, malformed JSON, shape mismatch
 * — is funneled through `request` and surfaces as `ApiError`.
 */
export async function postSimulate(
  request: CombatRequest,
): Promise<SimulationResponse> {
  const body = await jsonRequest(API_ROUTES.simulate, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });

  // A runtime guard that the body matches the expected shape. The compile-time
  // type is `SimulationResponse`; this catches a server returning, say, an
  // error object with a 200 status, without forcing the UI to handle `any`.
  if (!isSimulationResponse(body)) {
    throw new ApiError(
      `Response from ${API_ROUTES.simulate} did not match the expected shape`,
      0,
      API_ROUTES.simulate,
    );
  }

  return body;
}

/**
 * Shared fetch wrapper. Resolves to the parsed JSON body on a 2xx response;
 * throws `ApiError` for network failures, non-2xx responses, and bodies that
 * are not valid JSON.
 *
 * Kept generic over `T` so callers that want a plain `unknown` (a future health
 * route returning text, say) can have it without the JSON parse step changing.
 */
async function jsonRequest<T = unknown>(
  endpoint: string,
  init: RequestInit,
): Promise<T> {
  let response: Response;
  try {
    response = await fetch(apiUrl(endpoint), init);
  } catch (cause) {
    throw new ApiError(
      `Network error contacting ${endpoint}: ${formatCause(cause)}`,
      0,
      endpoint,
    );
  }

  if (!response.ok) {
    // combat-api returns plain text for client errors (e.g. "both fleets are
    // empty; there is nothing to simulate"). Prefer that text when present.
    let detail = "";
    try {
      detail = (await response.text()).trim();
    } catch {
      detail = "";
    }
    const message = detail || `${response.status} ${response.statusText}`;
    throw new ApiError(message, response.status, endpoint);
  }

  let body: T;
  try {
    body = (await response.json()) as T;
  } catch (cause) {
    throw new ApiError(
      `Response from ${endpoint} was not valid JSON: ${formatCause(cause)}`,
      response.status,
      endpoint,
    );
  }

  return body;
}

function formatCause(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  return String(cause);
}

function isSimulationResponse(value: unknown): value is SimulationResponse {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.results === "object" &&
    v.results !== null &&
    typeof v.report === "object" &&
    v.report !== null
  );
}
