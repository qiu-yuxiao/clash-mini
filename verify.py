import re
import os
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
    wait_interval_match = re.search(r"pub const SERVICE_WAIT_INTERVAL:\s*Duration\s*=\s*Duration::from_millis\((\d+)\);", content)
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
    # Check mixed port
    mixed_match = re.search(r"pub const DEFAULT_MIXED:\s*u16\s*=\s*(\d+);", content)
    if mixed_match:
        port = int(mixed_match.group(1))
        if port == 10801:
            print(f"[PASS] DEFAULT_MIXED port is 10801")
        else:
            print(f"[FAIL] DEFAULT_MIXED port is {port}, expected 10801")
            success = False
    else:
        print("[FAIL] DEFAULT_MIXED definition not found")
        success = False
        
    # Check controller API port
    controller_match = re.search(r"pub const DEFAULT_EXTERNAL_CONTROLLER:\s*&str\s*=\s*\"([^\"]+)\";", content)
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
        
    # Check singleton ports
    singleton_matches = re.findall(r"pub const SINGLETON_SERVER:\s*u16\s*=\s*(\d+);", content)
    if len(singleton_matches) == 2:
        ports = [int(p) for p in singleton_matches]
        if 33335 in ports and 33336 in ports:
            print(f"[PASS] SINGLETON_SERVER configured for 33335 (Release) and 33336 (Dev)")
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
    
    # Check retry_delay in ServiceManager config
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
    
    # Locate wait_for_service_if_needed
    # Check for admin status check directly in file content to avoid non-greedy brace matching issues
    admin_check_match = re.search(r"let\s+(\w+)\s*=\s*tauri_plugin_clash_verge_sysinfo::is_current_app_handle_admin\(Handle::app_handle\(\)\);", content)
    if admin_check_match:
        var_name = admin_check_match.group(1)
        print(f"[PASS] Found admin status check using variable: {var_name}")
        
        # Check for early return if admin is true
        return_match = re.search(r"if\s+" + var_name + r"\s*\{\s*return;?\s*\}", content)
        if return_match:
            print(f"[PASS] wait_for_service_if_needed returns immediately if {var_name} is true")
        else:
            print(f"[FAIL] wait_for_service_if_needed checks admin but does not return immediately")
            success = False
    else:
        print("[FAIL] is_current_app_handle_admin check not found in content")
        success = False
        
    return success

import json

import urllib.request

def get_latest_upstream_plugin_version():
    url = "https://raw.githubusercontent.com/clash-verge-rev/tauri-plugin-mihomo/main/Cargo.toml"
    req = urllib.request.Request(url)
    req.add_header("User-Agent", "verify-script")
    try:
        with urllib.request.urlopen(req, timeout=8) as response:
            content = response.read().decode()
            match = re.search(r"version\s*=\s*\"([^\"]+)\"", content)
            if match:
                return match.group(1)
    except Exception as e:
        print(f"[WARN] Failed to fetch latest Cargo.toml from GitHub raw: {e}")
    return None

def verify_cargo_lock():
    path = "Cargo.lock"
    if not os.path.exists(path):
        print(f"[FAIL] Cargo.lock not found at {path}")
        return False
        
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
        
    print("\n--- Verifying Cargo.lock Plugin Dependency ---")
    
    # Match package section for tauri-plugin-mihomo
    pattern = r"\[\[package\]\]\s+name\s*=\s*\"tauri-plugin-mihomo\"[\s\S]*?version\s*=\s*\"([^\"]+)\"[\s\S]*?source\s*=\s*\"([^\"]+)\""
    match = re.search(pattern, content)
    if not match:
        print("[FAIL] tauri-plugin-mihomo package entry not found in Cargo.lock")
        return False
        
    version = match.group(1)
    source = match.group(2)
    
    commit_match = re.search(r"#([a-fA-F0-9]+)", source)
    commit = commit_match.group(1) if commit_match else None
    
    print(f"[INFO] Found tauri-plugin-mihomo version: {version}, commit: {commit}")
    
    bad_commit = "e8f46f631f40259bcbe252f14dd49efd1afdc2f0"
    if commit and commit.lower() == bad_commit.lower():
        print(f"[FAIL] tauri-plugin-mihomo is locked to the bugged commit: {bad_commit} (v0.3.0)!")
        return False
        
    try:
        v_parts = [int(x) for x in version.split(".")]
        if v_parts < [0, 5, 2]:
            print(f"[FAIL] tauri-plugin-mihomo version is {version}, but must be >= 0.5.2 to include the white screen fix!")
            return False
    except Exception as e:
        print(f"[WARN] Failed to parse semver for {version}: {e}")
        
    # Check if there is a newer version upstream
    latest_upstream = get_latest_upstream_plugin_version()
    if latest_upstream:
        print(f"[INFO] Latest upstream plugin version found: {latest_upstream}")
        try:
            v_local = [int(x) for x in version.split(".")]
            v_upstream = [int(x) for x in latest_upstream.split(".")]
            if v_local < v_upstream:
                print(f"[FAIL] Local plugin version ({version}) is OUTDATED! Upstream has v{latest_upstream}.")
                print(f"[FAIL] Please run 'cargo update -p tauri-plugin-mihomo' to update your dependencies before releasing!")
                return False
            else:
                print(f"[PASS] Local version {version} is up-to-date with upstream v{latest_upstream}")
        except Exception as e:
            print(f"[WARN] Failed to compare local and upstream versions: {e}")
    else:
        print("[WARN] Could not retrieve latest upstream version, skipping remote check (using local offline rules)")
        
    print("[PASS] tauri-plugin-mihomo dependency is correctly updated and verified")
    return True

def verify_changelog_sync():
    package_path = "package.json"
    bug_list_path = "bug_list.md"
    changelog_path = "Changelog.md"
    
    if not os.path.exists(package_path):
        print(f"[FAIL] package.json not found")
        return False
    if not os.path.exists(bug_list_path):
        print(f"[FAIL] bug_list.md not found")
        return False
    if not os.path.exists(changelog_path):
        print(f"[FAIL] Changelog.md not found")
        return False
        
    with open(package_path, "r", encoding="utf-8") as f:
        pkg = json.load(f)
    current_version = pkg.get("version")
    if not current_version:
        print("[FAIL] Version field not found in package.json")
        return False
        
    print(f"\n--- Verifying Changelog & Bug List Alignment for version v{current_version} ---")
    
    resolved_bugs = []
    with open(bug_list_path, "r", encoding="utf-8") as f:
        bug_content = f.read()
        
    for line in bug_content.split("\n"):
        if line.strip().startswith("|") and "BUG-" in line:
            parts = [p.strip() for p in line.split("|")]
            if len(parts) >= 5:
                bug_id_raw = parts[1]
                resolve_ver_raw = parts[3]
                
                bug_id = re.sub(r"\*\*|\*", "", bug_id_raw).strip()
                resolve_ver = re.sub(r"^v", "", resolve_ver_raw).split("-")[0].strip()
                
                if resolve_ver == current_version:
                    resolved_bugs.append(bug_id)
                    
    print(f"[INFO] Resolved bugs listed for v{current_version} in bug_list.md: {resolved_bugs}")
    
    if not resolved_bugs:
        print(f"[WARN] No bugs resolved for version v{current_version} in bug_list.md. (Ensure this is expected)")
        
    with open(changelog_path, "r", encoding="utf-8") as f:
        changelog_content = f.read()
        
    lines = changelog_content.split("\n")
    changelog_section_lines = []
    is_capturing = False
    
    title_regex = re.compile(r"^##\s+v?" + re.escape(current_version) + r"\b", re.IGNORECASE)
    next_title_regex = re.compile(r"^##\s+v?\d+", re.IGNORECASE)
    
    for line in lines:
        if title_regex.match(line):
            is_capturing = True
            continue
        if is_capturing:
            if next_title_regex.match(line) or line.strip() == "## 原始版本历史 (Clash Verge History)":
                break
            changelog_section_lines.append(line)
            
    changelog_section = "\n".join(changelog_section_lines).strip()
    
    if not changelog_section:
        print(f"[FAIL] Changelog.md does not contain a section for v{current_version}!")
        return False
        
    success = True
    for bug_id in resolved_bugs:
        if bug_id not in changelog_section:
            print(f"[FAIL] {bug_id} is resolved in bug_list.md for v{current_version}, but NOT mentioned in Changelog.md v{current_version} section!")
            success = False
        else:
            print(f"[PASS] Mapped {bug_id} to Changelog description")
            
    if success:
        print(f"[PASS] All resolved bugs for v{current_version} are documented in Changelog.md")
        
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
