# Handoff Report: R3 Dependency and Build Consistency Audit

This handoff report documents the findings of the static audit for **R3: Dependency and Build Consistency** in the Clash Mini project.

---

## 1. Observation

### Observation A: `tauri-plugin-mihomo` Version Inconsistencies
We observed mismatches in the declared version of `tauri-plugin-mihomo` across files:
* In [crates/tauri-plugin-mihomo/package.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/package.json) (Line 3):
  ```json
  "version": "0.5.2",
  ```
* In [crates/tauri-plugin-mihomo/Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/Cargo.toml) (Line 3):
  ```toml
  version = "0.5.4"
  ```
* In [crates/tauri-plugin-mihomo/Cargo.lock](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/Cargo.lock) (Line 3629):
  ```toml
  version = "0.5.2"
  ```
* In the root workspace [Cargo.lock](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Cargo.lock) (Line 7724):
  ```toml
  version = "0.5.4"
  ```

### Observation B: Stale TypeScript Type Bindings for `FindProcessMode`
In the Rust source file [crates/tauri-plugin-mihomo/src/models.rs](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/src/models.rs) (Lines 446-447):
```rust
#[ts(export, rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FindProcessMode {
    Strict,
    Always,
    Off,
}
```
However, in the generated TypeScript bindings, the values are still capitalized (PascalCase):
* In [crates/tauri-plugin-mihomo/bindings/FindProcessMode.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/bindings/FindProcessMode.ts) (Line 3):
  ```typescript
  export type FindProcessMode = 'Strict' | 'Always' | 'Off'
  ```
* In the built output [crates/tauri-plugin-mihomo/dist-js/bindings/FindProcessMode.d.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/dist-js/bindings/FindProcessMode.d.ts) (Line 1):
  ```typescript
  export type FindProcessMode = 'Strict' | 'Always' | 'Off'
  ```

### Observation C: Build Script Discrepancy
* In [src-tauri/tauri.conf.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json) (Line 28):
  ```json
  "beforeBuildCommand": "pnpm vite build",
  ```
* In the root [package.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/package.json) (Line 15):
  ```json
  "web:build": "tsc --noEmit && vite build",
  ```
The build configuration in `tauri.conf.json` invokes `pnpm vite build` directly, bypassing the TypeScript static compilation type checks (`tsc --noEmit`) which are run in `pnpm web:build`.

---

## 2. Logic Chain

1. **Version Mismatch**: `tauri-plugin-mihomo` was bumped locally to version `0.5.4` in `Cargo.toml` (Observation A) to satisfy `verify.py` requirements. However, the crate's internal `package.json` and its `Cargo.lock` were not updated (Observation A).
2. **Stale Bindings**: The Rust struct `FindProcessMode` was patched with `#[ts(export, rename_all = "lowercase")]` (Observation B) to align the TS bindings with the deserialization requirements of the backend (since the backend uses `#[serde(rename_all = "lowercase")]` and will reject uppercase or PascalCase variants).
3. **Reason for Stale Bindings**: `tauri-plugin-mihomo` is not listed as a workspace member in the root [Cargo.toml](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/Cargo.toml) (Lines 2-9). Thus, executing cargo commands (such as `cargo test`) from the root workspace does not run tests in `crates/tauri-plugin-mihomo`.
4. **Impact on JS Compilation**: Since `ts-rs` only exports and updates the files in the `bindings/` folder during cargo tests, `FindProcessMode.ts` remained unregenerated (stale). Further, since rollup compiles from `guest-js/` using rollup config, the `dist-js/bindings/FindProcessMode.d.ts` file in the build distribution also remained stale (Observation B).
5. **Impact on Production Builds**: `beforeBuildCommand` in `tauri.conf.json` runs `pnpm vite build` directly without running type checks (Observation C). If any TypeScript compile error exists in the frontend code, it will pass silently during packaging.

---

## 3. Caveats

* **Local Command Execution**: We did not execute `cargo test` or `pnpm build` during this audit to verify if they succeed without compilation errors in the local plugin due to the read-only audit constraint.
* **Mihomo Sidecar Executables**: We observed that the `src-tauri/sidecar/` directory contains both the correct `mini-mihomo` sidecars and the old `verge-mihomo` sidecars. The `verge-mihomo` sidecars are unused because only `mini-mihomo` is registered in `tauri.conf.json` (Observation C/Constants). However, they take up disk space and should be cleaned up.

---

## 4. Conclusion

There are three major dependency and build inconsistencies in the repository:
1. **Version mismatch**: `crates/tauri-plugin-mihomo/package.json` and `crates/tauri-plugin-mihomo/Cargo.lock` are locked to `0.5.2`, while `Cargo.toml` is on `0.5.4`.
2. **Out-of-sync TS bindings**: The generated TypeScript binding `FindProcessMode` defines `'Strict' | 'Always' | 'Off'`, but the Rust backend strictly expects `'strict' | 'always' | 'off'`.
3. **Bypassed Type Checking**: Tauri's `beforeBuildCommand` runs `pnpm vite build` instead of `pnpm web:build`, skipping static type verification.

### Suggested Diffs

#### Diff A: Resolve `tauri-plugin-mihomo` package version
In [crates/tauri-plugin-mihomo/package.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/package.json):
```diff
--- c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/package.json
+++ c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/package.json
@@ -3,3 +3,3 @@
   "name": "tauri-plugin-mihomo-api",
-  "version": "0.5.2",
+  "version": "0.5.4",
   "author": "oomeow",
```

#### Diff B: Run Type Checks in Tauri Build Command
In [src-tauri/tauri.conf.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json):
```diff
--- c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json
+++ c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/src-tauri/tauri.conf.json
@@ -28,3 +28,3 @@
   "build": {
-    "beforeBuildCommand": "pnpm vite build",
+    "beforeBuildCommand": "pnpm web:build",
     "frontendDist": "../dist",
```

#### Diff C: Add Plugin Build Helper Script
In the root [package.json](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/package.json):
```diff
--- c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/package.json
+++ c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/package.json
@@ -17,2 +17,3 @@
     "prebuild": "node scripts/prebuild.mjs",
+    "plugin:build": "pnpm --dir crates/tauri-plugin-mihomo build",
     "updater": "node scripts/updater.mjs",
```

---

## 5. Verification Method

To verify these issues and their resolution:
1. **Version Verification**: Verify the version values in `package.json` and `Cargo.toml` in `crates/tauri-plugin-mihomo` match exactly.
2. **TS Bindings Regeneration & Compilation**:
   * Navigate to `crates/tauri-plugin-mihomo/` and run `cargo test` to compile and regenerate the ts-rs bindings.
   * Verify that [crates/tauri-plugin-mihomo/bindings/FindProcessMode.ts](file:///c:/Users/sun_y/Documents/AntiGravity_Projects/ClashVerge/crates/tauri-plugin-mihomo/bindings/FindProcessMode.ts) changes from PascalCase to lowercase string types:
     ```typescript
     export type FindProcessMode = 'strict' | 'always' | 'off'
     ```
   * Run `pnpm build` inside `crates/tauri-plugin-mihomo/` to compile the Rollup assets. Verify that `dist-js/bindings/FindProcessMode.d.ts` is updated accordingly.
3. **Verify Build Process**: Run `pnpm tauri build` or `pnpm web:build` and check that the typescript compiler (`tsc --noEmit`) runs successfully before compiling front-end assets.
