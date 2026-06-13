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

if __name__ == "__main__":
    c_ok = verify_constants()
    s_ok = verify_service()
    a_ok = verify_admin_check()
    
    if c_ok and s_ok and a_ok:
        print("\n=== ALL VERIFICATION CHECKS PASSED SUCCESSFULLY ===")
        sys.exit(0)
    else:
        print("\n=== SOME VERIFICATION CHECKS FAILED ===")
        sys.exit(1)
