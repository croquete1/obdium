// Reserve ownership before an asynchronous listener registration begins.
export function createTrackerRegistry(onCleanupError = console.error) {
  const active = new Map();

  function release(cleanup) {
    try {
      cleanup();
    } catch (error) {
      onCleanupError(error);
    }
  }

  function begin(key) {
    const previous = active.get(key);
    const cleanups = [];
    const ticket = {
      isCurrent: () => active.get(key) === ticket,
      attach(cleanup) {
        if (!ticket.isCurrent()) {
          release(cleanup);
          return false;
        }
        cleanups.push(cleanup);
        return true;
      },
      stop() {
        if (ticket.isCurrent()) active.delete(key);
        for (const cleanup of cleanups.splice(0).reverse()) release(cleanup);
      },
    };
    active.set(key, ticket);
    previous?.stop();
    return ticket;
  }

  return {
    begin,
    stopAll() {
      for (const ticket of [...active.values()]) ticket.stop();
    },
  };
}

// Delayed queries belong only to the connection that scheduled them.
export async function runConnectionQueries({ isCurrent, delay, emit, firstDelay }) {
  await delay(firstDelay);
  if (!isCurrent()) return;
  emit("get-pids");
  await delay(2000);
  if (!isCurrent()) return;
  emit("get-readiness-tests");
}
