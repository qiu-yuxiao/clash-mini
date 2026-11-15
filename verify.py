import json
import os
import re
import sys


def verify_constants():
    path = os.path.join("src-tauri", "src", "constants.rs")
    if not os.path.exists(path):
        print(f"[FAIL] constants.rs not found at {path}")
        return False

    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    success = True
    print("\n--- Verifying Constants and Ports ---")

    # 1. Check SERVICE_WAIT_INTERVAL
    wait_interval_match = re.search(
        r"pub const SERVICE_WAIT_INTERVAL:\s*Duration\s*=\s*Duration::from_millis\((\d+)\);",
        content,
    )
    if wait_interval_match:
        val = int(wait_interval_match.group(1))
        if val > 0:
            print(f"[PASS] SERVICE_WAIT_INTERVAL is non-zero: {val}ms")
        else:
            print("[FAIL] SERVICE_WAIT_INTERVAL is zero!")
            success = False
    else:
        print("[FAIL] SERVICE_WAIT_INTERVAL constant definition not found")
        success = False

    # 2. Check ports in clash_mini_agreements.md (10801 mixed, 9098 controller API, 33335/33336 singleton ports)
    mixed_match = re.search(r"pub const DEFAULT_MIXED:\s*u16\s*=\s*(\d+);", content)
    if mixed_match:
        port = int(mixed_match.group(1))
        if port == 10801:
            print("[PASS] DEFAULT_MIXED port is 10801")
        else:
            print(f"[FAIL] DEFAULT_MIXED port is {port}, expected 10801")
            success = False
    else:
        print("[FAIL] DEFAULT_MIXED definition not found")
        success = False

    controller_match = re.search(
        r'pub const DEFAULT_EXTERNAL_CONTROLLER:\s*&str\s*=\s*"([^"]+)";', content
    )
    if controller_match:
        addr = controller_match.group(1)
        if "9098" in addr:
            print(f"[PASS] DEFAULT_EXTERNAL_CONTROLLER port is 9098 ({addr})")
        else:
            print(f"[FAIL] DEFAULT_EXTERNAL_CONTROLLER is {addr}, expected to contain 9098")
            success = False
    else:
        print("[FAIL] DEFAULT_EXTERNAL_CONTROLLER definition not found")
        success = False

    singleton_matches = re.findall(r"pub const SINGLETON_SERVER:\s*u16\s*=\s*(\d+);", content)
    if len(singleton_matches) == 2:
        ports = [int(p) for p in singleton_matches]
        if 33335 in ports and 33336 in ports:
            print("[PASS] SINGLETON_SERVER configured for 33335 (Release) and 33336 (Dev)")
        else:
            print(f"[FAIL] SINGLETON_SERVER configured with {ports}, expected 33335 and 33336")
            success = False
    else:
        print(f"[FAIL] Found {len(singleton_matches)} SINGLETON_SERVER definitions, expected 2")
        success = False

    return success


def verify_service():
    path = os.path.join("src-tauri", "src", "core", "service.rs")
    if not os.path.exists(path):
        print(f"[FAIL] service.rs not found at {path}")
        return False

    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    success = True
    print("\n--- Verifying Service Configurations ---")

    retry_delay_match = re.search(r"retry_delay:\s*Duration::from_millis\((\d+)\)", content)
    if retry_delay_match:
        val = int(retry_delay_match.group(1))
        if val > 0:
            print(f"[PASS] ServiceManager retry_delay is non-zero: {val}ms")
        else:
            print("[FAIL] ServiceManager retry_delay is zero!")
            success = False
    else:
        print("[FAIL] ServiceManager retry_delay definition not found")
        success = False

    return success


def verify_admin_check():
    path = os.path.join("src-tauri", "src", "core", "manager", "lifecycle.rs")
    if not os.path.exists(path):
        print(f"[FAIL] lifecycle.rs not found at {path}")
        return False

    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    success = True
    print("\n--- Verifying Admin Loop-Skipping ---")

    fn_match = re.search(r"async fn wait_for_service_ready\b[\s\S]*?\n    \}", content)
    if not fn_match:
        print("[FAIL] wait_for_service_ready function block not found in lifecycle.rs")
        return False

    fn_body = fn_match.group(0)
    admin_check_match = re.search(
        r"let\s+(\w+)\s*=\s*crate::utils::sysinfo::is_current_app_handle_admin\(Handle::app_handle\(\)\);",
        fn_body,
    )
    if admin_check_match:
        var_name = admin_check_match.group(1)
        print(f"[PASS] Found admin status check in wait_for_service_ready: {var_name}")

        return_match = re.search(r"if\s+" + re.escape(var_name) + r"\s*\{\s*return;?\s*\}", fn_body)
        if return_match:
            print(f"[PASS] wait_for_service_ready returns immediately if {var_name} is true")
        else:
            print("[FAIL] wait_for_service_ready checks admin but does not return immediately")
            success = False
    else:
        print("[FAIL] is_current_app_handle_admin check not found in wait_for_service_ready")
        success = False

    return success


def verify_cargo_lock():
    print("\n--- Verifying Local Mihomo Plugin & Dependency Lock ---")
    success = True

    cargo_toml_path = os.path.join("src-tauri", "Cargo.toml")
    if not os.path.exists(cargo_toml_path):
        print(f"[FAIL] {cargo_toml_path} not found")
        return False
    with open(cargo_toml_path, "r", encoding="utf-8") as f:
        cargo_toml = f.read()

    if re.search(
        r'tauri-plugin-mihomo\s*=\s*\{\s*path\s*=\s*"\.\./crates/tauri-plugin-mihomo"\s*\}',
        cargo_toml,
    ):
        print('[PASS] src-tauri/Cargo.toml uses local path "../crates/tauri-plugin-mihomo"')
    else:
        print("[FAIL] src-tauri/Cargo.toml does not point tauri-plugin-mihomo to local path crate")
        success = False

    lock_path = "Cargo.lock"
    if not os.path.exists(lock_path):
        print(f"[FAIL] Cargo.lock not found at {lock_path}")
        return False
    with open(lock_path, "r", encoding="utf-8") as f:
        lock_content = f.read()

    blocks = lock_content.split("[[package]]")
    mihomo_block = next((b for b in blocks if 'name = "tauri-plugin-mihomo"' in b), None)
    if not mihomo_block:
        print("[FAIL] tauri-plugin-mihomo package entry not found in Cargo.lock")
        return False

    version_match = re.search(r'version\s*=\s*"([^"]+)"', mihomo_block)
    if not version_match:
        print("[FAIL] version not found in tauri-plugin-mihomo block")
        return False
    version = version_match.group(1)

    source_match = re.search(r'source\s*=\s*"([^"]+)"', mihomo_block)
    source = source_match.group(1) if source_match else None
    commit = None
    if source:
        commit_match = re.search(r"#([a-fA-F0-9]+)", source)
        commit = commit_match.group(1) if commit_match else None

    print(f"[INFO] Found tauri-plugin-mihomo version: {version}, source: {source or 'local-path'}")

    bad_commit = "e8f46f631f40259bcbe252f14dd49efd1afdc2f0"
    if commit and commit.lower() == bad_commit.lower():
        print(f"[FAIL] tauri-plugin-mihomo is locked to bugged commit {bad_commit}!")
        return False

    try:
        v_parts = [int(x) for x in version.split(".")[:3]]
        if v_parts < [0, 5, 2]:
            print(f"[FAIL] tauri-plugin-mihomo version is {version}, expected >= 0.5.2")
            return False
    except Exception as e:
        print(f"[FAIL] Failed to parse semver for tauri-plugin-mihomo ({version}): {e}")
        return False

    models_path = os.path.join("crates", "tauri-plugin-mihomo", "src", "models.rs")
    if not os.path.exists(models_path):
        print(f"[FAIL] {models_path} not found")
        return False
    with open(models_path, "r", encoding="utf-8") as f:
        models_content = f.read()

    for enum_name in ("LogLevel", "FindProcessMode"):
        pattern = (
            r'#\[serde\(rename_all\s*=\s*"lowercase"\)\]\s*'
            r'(?:#\[[^\]]+\]\s*)*pub\s+enum\s+' + enum_name + r"\b"
        )
        if re.search(pattern, models_content):
            print(f'[PASS] {enum_name} keeps #[serde(rename_all = "lowercase")] patch (BUG-072)')
        else:
            print(f'[FAIL] {enum_name} missing #[serde(rename_all = "lowercase")] in models.rs')
            success = False

    lib_path = os.path.join("src-tauri", "src", "lib.rs")
    if not os.path.exists(lib_path):
        print(f"[FAIL] {lib_path} not found")
        return False
    with open(lib_path, "r", encoding="utf-8") as f:
        lib_content = f.read()

    if "Protocol::LocalSocket" in lib_content and "RejectPolicy::Timeout(" in lib_content:
        print("[PASS] lib.rs configures Protocol::LocalSocket and RejectPolicy::Timeout")
    else:
        print("[FAIL] lib.rs missing Protocol::LocalSocket or RejectPolicy::Timeout")
        success = False

    if success:
        print("[PASS] Local tauri-plugin-mihomo dependency & IPC configuration verified offline")
    return success


def verify_changelog_sync():
    package_path = "package.json"
    bug_list_path = "bug_list.md"
    changelog_path = "Changelog.md"
    agreements_path = "clash_mini_agreements.md"

    for p in (package_path, bug_list_path, changelog_path, agreements_path):
        if not os.path.exists(p):
            print(f"[FAIL] {p} not found")
            return False

    with open(package_path, "r", encoding="utf-8") as f:
        pkg = json.load(f)
    current_version = pkg.get("version")
    if not current_version:
        print("[FAIL] Version field not found in package.json")
        return False

    print(f"\n--- Verifying Changelog & Bug List Alignment for v{current_version} ---")

    with open(bug_list_path, "r", encoding="utf-8") as f:
        bug_content = f.read()
    with open(agreements_path, "r", encoding="utf-8") as f:
        agreements_content = f.read()

    archived_bugs_for_version = []
    archived_bug_ids = set()
    for line in bug_content.split("\n"):
        if line.strip().startswith("|") and "BUG-" in line:
            parts = [p.strip() for p in line.split("|")]
            if len(parts) >= 4:
                bug_id = re.sub(r"\*", "", parts[1]).strip()
                if not bug_id.startswith("BUG-"):
                    continue
                archived_bug_ids.add(bug_id)
                resolve_ver = re.sub(r"^v", "", parts[2]).split("-")[0].strip()
                if resolve_ver == current_version:
                    archived_bugs_for_version.append(bug_id)

    pending_bug_ids = set(re.findall(r"^###\s+(BUG-\d+)\b", bug_content, re.M))
    known_bug_list_ids = archived_bug_ids | pending_bug_ids
    agreements_bug_ids = set(re.findall(r"\|\s*\*\*(BUG-\d+)\*\*\s*\|", agreements_content))

    print(f"[INFO] Archived bugs for v{current_version} in bug_list.md: {archived_bugs_for_version}")
    print(f"[INFO] Pending bugs indexed in bug_list.md: {len(pending_bug_ids)}")

    with open(changelog_path, "r", encoding="utf-8") as f:
        changelog_content = f.read()

    changelog_section_lines = []
    is_capturing = False
    title_regex = re.compile(r"^##\s+v?" + re.escape(current_version) + r"\b", re.IGNORECASE)
    next_title_regex = re.compile(r"^##\s+v?\d+", re.IGNORECASE)

    for line in changelog_content.split("\n"):
        if title_regex.match(line):
            is_capturing = True
            continue
        if is_capturing:
            if (
                next_title_regex.match(line)
                or line.strip() == "## 原始版本历史 (Clash Verge History)"
            ):
                break
            changelog_section_lines.append(line)

    changelog_section = "\n".join(changelog_section_lines).strip()
    if not changelog_section:
        print(f"[FAIL] Changelog.md does not contain a section for v{current_version}!")
        return False
    print(f"[PASS] Changelog.md contains section for v{current_version}")

    success = True
    for bug_id in archived_bugs_for_version:
        if bug_id not in changelog_section:
            print(
                f"[FAIL] {bug_id} is archived for v{current_version} in bug_list.md, "
                f"but NOT mentioned in Changelog.md v{current_version} section!"
            )
            success = False
        else:
            print(f"[PASS] Mapped archived {bug_id} to Changelog v{current_version}")

    heading_bugs = []
    for line in changelog_section_lines:
        if line.strip().startswith("###"):
            heading_bugs.extend(re.findall(r"\bBUG-\d+\b", line))

    for bug_id in dict.fromkeys(heading_bugs):
        if bug_id not in known_bug_list_ids:
            print(f"[FAIL] {bug_id} cited in Changelog v{current_version} heading missing from bug_list.md")
            success = False
        elif bug_id not in agreements_bug_ids:
            print(
                f"[FAIL] {bug_id} cited in Changelog v{current_version} heading missing from "
                "clash_mini_agreements.md Appendix A"
            )
            success = False
        else:
            print(f"[PASS] Verified heading {bug_id} in bug_list.md and Appendix A")

    all_cited_bugs = set(re.findall(r"\bBUG-\d+\b", changelog_section))
    for bug_id in sorted(all_cited_bugs):
        if bug_id not in known_bug_list_ids and bug_id not in agreements_bug_ids:
            print(f"[FAIL] {bug_id} mentioned in Changelog v{current_version} is unknown everywhere")
            success = False

    if success:
        print(f"[PASS] All v{current_version} changelog & bug references verified")
    return success


if __name__ == "__main__":
    c_ok = verify_constants()
    s_ok = verify_service()
    a_ok = verify_admin_check()
    dep_ok = verify_cargo_lock()
    ch_ok = verify_changelog_sync()

    if c_ok and s_ok and a_ok and dep_ok and ch_ok:
        print("\n=== ALL VERIFICATION CHECKS PASSED SUCCESSFULLY ===")
        sys.exit(0)
    else:
        print("\n=== SOME VERIFICATION CHECKS FAILED ===")
        sys.exit(1)
