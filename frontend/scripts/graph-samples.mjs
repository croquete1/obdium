// Graph samples must represent observed measurements, not fabricated zeros.
// Bound time spent in the frontend queue; this is not ECU observation age.
const MAX_PENDING_AGE_MS = 2000;

export function createGraphSampler(now = () => performance.now()) {
  let pending = null;
  return {
    push(payload) {
      const unit = typeof payload?.unit === "string"
        ? payload.unit.trim().toUpperCase()
        : null;
      const receivedAt = now();
      pending = unit !== null && unit !== "NO DATA" &&
        Number.isFinite(payload?.value) && Number.isFinite(receivedAt)
        ? { sample: { value: payload.value, unit }, receivedAt }
        : null;
    },
    take() {
      const entry = pending;
      pending = null;
      if (!entry) return null;
      const age = now() - entry.receivedAt;
      return Number.isFinite(age) && age >= 0 && age < MAX_PENDING_AGE_MS
        ? entry.sample
        : null;
    },
  };
}
