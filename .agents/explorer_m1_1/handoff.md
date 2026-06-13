# Handoff Report - Explorer 1 (Milestone 1)

## 1. Observation
- **File Checked**: `src/hooks/use-visibility.ts`
- **Current Hook Code**:
  ```typescript
  export const useVisibility = () => {
    const [visible, setVisible] = useState(() =>
      typeof document === 'undefined'
        ? true
        : document.visibilityState === 'visible',
    )

    useEffect(() => {
      const handleVisibilityChange = () => {
        setVisible(document.visibilityState === 'visible')
      }

      const handleFocus = () => setVisible(true)
      const handlePointerDown = () => setVisible(true)

      document.addEventListener('focus', handleFocus)
      document.addEventListener('pointerdown', handlePointerDown)
      document.addEventListener('visibilitychange', handleVisibilityChange)

      return () => {
        document.removeEventListener('focus', handleFocus)
        document.removeEventListener('pointerdown', handlePointerDown)
        document.removeEventListener('visibilitychange', handleVisibilityChange)
      }
    }, [])

    return visible
  }
  ```
- **Tauri API Definitions**: Found in `node_modules/@tauri-apps/api/window.d.ts` (lines 361, 383, 1269, 1184):
  - `isMinimized(): Promise<boolean>;`
  - `isFocused(): Promise<boolean>;`
  - `onResized(handler: EventCallback<PhysicalSize>): Promise<UnlistenFn>;`
  - `onFocusChanged(handler: EventCallback<boolean>): Promise<UnlistenFn>;`
- **Scope Contract**: Defined in `.agents/sub_orch_m1/SCOPE.md` (lines 15-17):
  ```markdown
  - `useVisibility()` returns true if and only if:
    1. `document.visibilityState === 'visible'`
    2. The window is not minimized (checked asynchronously via `getCurrentWindow().isMinimized()`).
  ```

## 2. Logic Chain
1. The original implementation of `useVisibility` checks only standard DOM focus/click events and `document.visibilityState` (Observation 1).
2. It lacks any detection of Tauri-native window states like OS-level minimization or hiding (Observation 1).
3. The Tauri v2 window API offers native helper methods: `isMinimized()` for state checks, and event hooks `onResized` and `onFocusChanged` to catch window minimized/restored/focused changes (Observation 2).
4. By combining `document.visibilityState` with an asynchronous call to `currentWindow.isMinimized()` inside event listeners (both DOM and Tauri-native window events), we can fully satisfy the contract requirements specified in `SCOPE.md` (Observation 3).
5. Wrapping Tauri calls in `try...catch` blocks is necessary to prevent crashes when executing in standard browser mock environments or during unit tests.

## 3. Caveats
- **Focus vs. Minimization**: While the prompt mentions "minimized/focused states", the explicit interface contract in `SCOPE.md` only mandates checking if the window is minimized. We choose not to enforce `isFocused` as a strict visibility constraint (making visibility false on blur) because it would disconnect WebSockets whenever Clash Mini loses focus (e.g. side-by-side active monitoring), which degrades the user experience. However, we still listen to native focus change events as a trigger to re-evaluate minimized/visibility states.

## 4. Conclusion
We recommend updating `src/hooks/use-visibility.ts` to include:
- A local `try...catch` wrapper on `getCurrentWindow()` to allow running in browser/test environments.
- Tauri-native `onResized` and `onFocusChanged` window listeners that trigger asynchronous visibility evaluations.
- Evaluated visibility returning `true` if and only if `document.visibilityState === 'visible'` AND `!isMinimized` (asynchronous state).

The detailed implementation strategy and code proposal have been saved to `analysis.md` in this directory.

## 5. Verification Method
- **Implementation Inspection**: Confirm that `src/hooks/use-visibility.ts` properly imports `getCurrentWindow` from `@tauri-apps/api/window`, registers the event hooks, and asynchronously queries `isMinimized()`.
- **Test Build Execution**: Run `pnpm typecheck` to verify TypeScript compilation and syntax soundness.
- **Behavioral Verification**: Verify that when the Clash Mini window is minimized, `useVisibility` yields `false` and connection/traffic WebSockets disconnect. When restored, they should reconnect.
