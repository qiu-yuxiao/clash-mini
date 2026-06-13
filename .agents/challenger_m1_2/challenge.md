## Challenge Summary

**Overall risk assessment**: LOW

## Challenges

### [Low] Challenge 1: Web Worker Termination State Loss

- **Assumption challenged**: The system relies on inline monitor mode (`this.startInline(initMessage)`) which is currently hardcoded in `TrafficWorkerClient.start()`.
- **Attack scenario**: If a future update changes the client to use `worker` mode (`this.mode = 'worker'`), calling `client.stop()` will execute `this.worker.terminate()`, which completely destroys the Web Worker thread and its memory. Upon subsequent `start()`, a new Web Worker will be spawned and initialized with an empty sampler, resulting in complete history data loss.
- **Blast radius**: Complete loss of traffic history data when switching to Web Worker mode on visibility transitions (minimized/restored).
- **Mitigation**: If Web Worker mode is restored, store the data sampler state in the main thread (on the `TrafficWorkerClient` itself) and send it back to the new worker on `init`, or suspend the worker instead of terminating it.

## Stress Test Results

- **Simulation Script**: A standalone node simulation script `scratch/verify_traffic_preservation.ts` was implemented to verify state preservation:
  1. Instantiated `TrafficWorkerClient`.
  2. Registered listener for snapshots.
  3. Appended sample data points `{ up: 100, down: 200 }`, `{ up: 150, down: 250 }`, `{ up: 200, down: 300 }`.
  4. Captured initial snapshot and verified 3 data points.
  5. Called `client.stop()` (suspending/stopping the client).
  6. Called `client.start()` (restarting the client).
  7. Requested snapshot and verified that the 3 historical data points are still present and have not been cleared.
- **Run Status**: Awaiting command execution permission approval (execution prompt timed out).
- **Predicted / Logical Verification**: PASS.
  - `TrafficWorkerClient.stop()` preserves the `inlineMonitor` instance (it is no longer set to `null`).
  - `TrafficWorkerClient.startInline()` reuses the existing `inlineMonitor` instance instead of creating a new one.
  - `InlineTrafficMonitor.stop()` does not clear the sampler (the `this.sampler.clear()` call was removed).
  - `InlineTrafficMonitor.handle('init')` preserves the sampler if it already exists (`if (!this.sampler)`).

## Unchallenged Areas

- **Web Worker Mode**: Out of scope for current active execution because the codebase hardcodes `startInline` to ensure stability.
