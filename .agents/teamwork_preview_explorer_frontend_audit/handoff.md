# Handoff Report: Frontend Audit of Clash Mini Changes

## 1. Observation
We analyzed the git diff of the requested files between `d3831a0ce5ecc6b2c040368570773f2622d0b91b` and `196e7c01`:
* **src/pages/_layout.tsx**: Found unstable dependencies (`refreshProxy`, `setHeadStateForSort`, `t`) in the profile enhancement `useEffect` (lines 1114-1145), module-scoped variable `activeAutoSelectTimer` with no cancellation on profile switches (lines 220-305), double selection calling `selectNodeForGroup` in both temporary and final blocks (lines 247-295), and unresolved promise leak.
* **src/services/delay.ts**: Found `Promise.race` (lines 225-228) timeout implementation without catching losing promise rejections.
* **src/utils/button-styles.ts**: Found hardcoded white transparent backgrounds (`rgba(255, 255, 255, 0.03)`) for disabled state (`Mui-disabled`) in the `frosted-glass` skin (lines 324-332).
* **crates/tauri-plugin-mihomo/guest-js/index.ts**: Deprecation notice added, no bugs.

## 2. Logic Chain
1. **Profile Activation Cancellation (src/pages/_layout.tsx):**
   - Unstable deps in `useEffect` trigger cleanup -> `cancelled = true`.
   - Promise gets cancelled.
   - Ref check `lastEnhancedProfileRef.current !== currentProfileUid` prevents it from restarting.
   - **Conclusion:** Profile activation/auto-selection gets permanently aborted.
2. **Profile Switch Race (src/pages/_layout.tsx):**
   - Profile switch does not clear `activeAutoSelectTimer`.
   - The interval callback continues to poll.
   - **Conclusion:** Selecting nodes on the previous profile overwrites settings of the new profile.
3. **Double Selection Race (src/pages/_layout.tsx):**
   - If `isFinalSelection` is met on the first tick, both temporary and final selection blocks run.
   - **Conclusion:** Duplicate concurrent backend requests lead to race conditions.
4. **Unresolved Promise Leak (src/pages/_layout.tsx):**
   - Concurrent calls clear the timer but never resolve/reject the previous Promise.
   - **Conclusion:** Memory leak of pending Promise objects.
5. **Unhandled Rejection (src/services/delay.ts):**
   - `delayProxyByName` in `Promise.race` continues running after timeout.
   - **Conclusion:** If it fails after timeout, it triggers an unhandled promise rejection.
6. **Low Contrast / Invisible Borders (src/utils/button-styles.ts):**
   - Frosted glass disabled button state uses transparent white on light themes.
   - **Conclusion:** Buttons look borderless/invisible with extremely low contrast text.

## 3. Caveats
No caveats. The investigation was strictly read-only and scoped to the four requested files.

## 4. Conclusion
We recommend implementing the stable refs pattern for React hook dependencies, passing cancellation functions to `frontendAutoSelect`, adding `.catch()` inside `Promise.race` for the delay measurement timeouts, and adding theme-conditional border/background colors for disabled states under the `frosted-glass` skin.

## 5. Verification Method
1. Verify by applying the diff patches provided in `analysis.md`.
2. Test profile switching quickly to confirm active nodes are not overwritten.
3. Check the DevTools console under high latency/network loss to ensure no unhandled promise rejections are thrown.
4. Change skin to `frosted-glass` in light mode, verify disabled button borders are visible.
