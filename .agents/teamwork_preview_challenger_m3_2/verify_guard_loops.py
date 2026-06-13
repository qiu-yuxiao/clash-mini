import re
import os
import sys

def verify_constants(constants_path):
    print(f"=== Verifying Constants in {constants_path} ===")
    if not os.path.exists(constants_path):
        print(f"ERROR: File not found at {constants_path}")
        return False
    
    with open(constants_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Verify SERVICE_WAIT_INTERVAL is non-zero
    wait_interval_match = re.search(r'pub\s+const\s+SERVICE_WAIT_INTERVAL\s*:\s*Duration\s*=\s*Duration::from_millis\((\d+)\);', content)
    if not wait_interval_match:
        print("ERROR: SERVICE_WAIT_INTERVAL not found or does not match pattern.")
        return False
    
    wait_interval_ms = int(wait_interval_match.group(1))
    print(f"Found SERVICE_WAIT_INTERVAL: {wait_interval_ms} ms")
    if wait_interval_ms <= 0:
        print("ERROR: SERVICE_WAIT_INTERVAL is zero or negative!")
        return False
    else:
        print("PASS: SERVICE_WAIT_INTERVAL is non-zero.")

    # 2. Verify DEFAULT_MIXED port
    mixed_port_match = re.search(r'pub\s+const\s+DEFAULT_MIXED\s*:\s*u16\s*=\s*(\d+);', content)
    if not mixed_port_match:
        print("ERROR: DEFAULT_MIXED port not found.")
        return False
    mixed_port = int(mixed_port_match.group(1))
    print(f"Found DEFAULT_MIXED port: {mixed_port}")
    if mixed_port != 10801:
        print("ERROR: DEFAULT_MIXED port is not 10801!")
        return False
    else:
        print("PASS: DEFAULT_MIXED port strictly matches 10801.")

    # 3. Verify DEFAULT_EXTERNAL_CONTROLLER port
    controller_match = re.search(r'pub\s+const\s+DEFAULT_EXTERNAL_CONTROLLER\s*:\s*&str\s*=\s*"[^"]+:(\d+)";', content)
    if not controller_match:
        print("ERROR: DEFAULT_EXTERNAL_CONTROLLER port not found.")
        return False
    controller_port = int(controller_match.group(1))
    print(f"Found DEFAULT_EXTERNAL_CONTROLLER port: {controller_port}")
    if controller_port != 9098:
        print("ERROR: DEFAULT_EXTERNAL_CONTROLLER port is not 9098!")
        return False
    else:
        print("PASS: DEFAULT_EXTERNAL_CONTROLLER port strictly matches 9098.")

    # 4. Verify SINGLETON_SERVER ports (Release & Dev)
    singleton_matches = re.findall(r'pub\s+const\s+SINGLETON_SERVER\s*:\s*u16\s*=\s*(\d+);', content)
    if len(singleton_matches) != 2:
        print(f"ERROR: Expected 2 SINGLETON_SERVER definitions, found {len(singleton_matches)}")
        return False
    
    singleton_release = int(singleton_matches[0])
    singleton_dev = int(singleton_matches[1])
    print(f"Found SINGLETON_SERVER (Release): {singleton_release}")
    print(f"Found SINGLETON_SERVER (Dev): {singleton_dev}")

    if singleton_release != 33335 or singleton_dev != 33336:
        print("ERROR: Singleton ports must be 33335 (Release) and 33336 (Dev)!")
        return False
    else:
        print("PASS: Singleton ports strictly match 33335/33336.")

    return True

def verify_service(service_path):
    print(f"\n=== Verifying Service Config in {service_path} ===")
    if not os.path.exists(service_path):
        print(f"ERROR: File not found at {service_path}")
        return False

    with open(service_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Verify retry_delay is non-zero
    retry_delay_match = re.search(r'retry_delay\s*:\s*Duration::from_millis\((\d+)\)', content)
    if not retry_delay_match:
        print("ERROR: retry_delay not found in service.rs")
        return False
    
    retry_delay_ms = int(retry_delay_match.group(1))
    print(f"Found retry_delay: {retry_delay_ms} ms")
    if retry_delay_ms <= 0:
        print("ERROR: retry_delay is zero or negative!")
        return False
    else:
        print("PASS: retry_delay is non-zero.")

    return True

def verify_lifecycle_admin_check(lifecycle_path):
    print(f"\n=== Verifying Admin Check Skip in {lifecycle_path} ===")
    if not os.path.exists(lifecycle_path):
        print(f"ERROR: File not found at {lifecycle_path}")
        return False

    with open(lifecycle_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Look for wait_for_service_if_needed definition
    if "async fn wait_for_service_if_needed" not in content:
        print("ERROR: wait_for_service_if_needed function not found!")
        return False

    # Extract wait_for_service_if_needed function body or surrounding lines
    lines = content.splitlines()
    func_start_idx = -1
    for idx, line in enumerate(lines):
        if "async fn wait_for_service_if_needed" in line:
            func_start_idx = idx
            break

    if func_start_idx == -1:
        print("ERROR: Could not locate start of wait_for_service_if_needed.")
        return False

    # Check for is_current_app_handle_admin check inside wait_for_service_if_needed
    # We examine the next 25 lines
    relevant_code = "\n".join(lines[func_start_idx:func_start_idx+25])
    print("Relevant code snippet from wait_for_service_if_needed:")
    print("--------------------------------------------------")
    print(relevant_code)
    print("--------------------------------------------------")

    admin_check_present = "is_current_app_handle_admin" in relevant_code
    early_return_present = re.search(r'if\s+is_admin\s*{\s*return;\s*}', relevant_code)

    if not admin_check_present:
        print("ERROR: is_current_app_handle_admin check is missing!")
        return False
    
    if not early_return_present:
        print("ERROR: Early return when is_admin is true is missing!")
        return False

    print("PASS: is_current_app_handle_admin check and early return are correctly implemented.")
    return True

if __name__ == "__main__":
    workspace_root = r"c:\Users\sun_y\Documents\AntiGravity_Projects\ClashVerge"
    constants_file = os.path.join(workspace_root, "src-tauri", "src", "constants.rs")
    service_file = os.path.join(workspace_root, "src-tauri", "src", "core", "service.rs")
    lifecycle_file = os.path.join(workspace_root, "src-tauri", "src", "core", "manager", "lifecycle.rs")

    ok = True
    ok &= verify_constants(constants_file)
    ok &= verify_service(service_file)
    ok &= verify_lifecycle_admin_check(lifecycle_file)

    if ok:
        print("\nAll verification checks PASSED successfully.")
        sys.exit(0)
    else:
        print("\nVerification checks FAILED.")
        sys.exit(1)
