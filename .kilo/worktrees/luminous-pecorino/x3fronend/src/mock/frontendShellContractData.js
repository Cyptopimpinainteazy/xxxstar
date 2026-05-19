import frontendRouteAllowlistArtifact from '../../../.launchops/frontend_route_allowlist.json';

function normalizeRouteContract(entry) {
  const allowedMethods = Array.isArray(entry.allowed_methods) ? [...entry.allowed_methods] : [];

  return {
    routeId: entry.route_id,
    routeLabel: entry.route_label,
    rationale: entry.rationale,
    allowedMethods,
    directReadCount: allowedMethods.length,
    enforcementMode: allowedMethods.length > 0 ? 'direct-read-guarded' : 'sidecar-only'
  };
}

const normalizedRouteContracts = (frontendRouteAllowlistArtifact.routes ?? []).map(normalizeRouteContract);

const routeContractsById = new Map(
  normalizedRouteContracts.map((entry) => [entry.routeId, entry])
);

export const shellRouteContracts = normalizedRouteContracts;

export function getRouteContract(routeId) {
  return (
    routeContractsById.get(routeId) ?? {
      routeId,
      routeLabel: routeId,
      rationale: 'No generated route contract entry is available for this shell route yet.',
      allowedMethods: [],
      directReadCount: 0,
      enforcementMode: 'sidecar-only'
    }
  );
}