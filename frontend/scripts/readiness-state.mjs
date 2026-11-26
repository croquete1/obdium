// Preserve diagnostic meaning independently of translated or styled display labels.
// Legacy backend "Unknown" rows represent no data, not unsupported monitors.
let snapshot = [];

export function normalizeReadinessTest(test) {
  const name = typeof test?.name === "string" && test.name.trim()
    ? test.name.trim() : "Unknown";
  if (name.toLowerCase() === "unknown" || name.toLowerCase() === "reserved"
      || typeof test?.available !== "boolean") {
    return { name, available: null, complete: null, status: "unknown" };
  }
  if (!test.available) {
    return { name, available: false, complete: null, status: "unsupported" };
  }
  if (typeof test.complete !== "boolean") {
    return { name, available: true, complete: null, status: "unknown" };
  }
  return {
    name, available: true, complete: test.complete,
    status: test.complete ? "complete" : "incomplete",
  };
}

export function readinessLabels(test) {
  return {
    availability: test.available === true ? "SUPPORTED"
      : test.available === false ? "UNSUPPORTED" : "UNKNOWN",
    completeness: test.status === "complete" ? "READY"
      : test.status === "incomplete" ? "NOT READY"
      : test.status === "unsupported" ? "NOT APPLICABLE" : "UNKNOWN",
  };
}

export function replaceReadinessSnapshot(payload) {
  snapshot = Array.isArray(payload) ? payload.map(normalizeReadinessTest) : [];
  return getReadinessSnapshot();
}

export function getReadinessSnapshot() {
  return snapshot.map((test) => ({ ...test }));
}
