#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::CoreUpdater;
use super::pe_verify::CoreBinaryValidation;
use sha2::{Digest as _, Sha256};
use std::fs;
use std::io::{self, Write as _};

fn build_minimal_valid_pe64_buffer(size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    // DOS Header
    buf[0] = b'M';
    buf[1] = b'Z';
    let e_lfanew = 128usize;
    buf[0x3C..0x40].copy_from_slice(&(e_lfanew as u32).to_le_bytes());

    // NT Signature: "PE\0\0"
    buf[e_lfanew..e_lfanew + 4].copy_from_slice(b"PE\0\0");
    // COFF FileHeader (20 bytes at e_lfanew + 4)
    let coff = e_lfanew + 4;
    buf[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes()); // Machine = AMD64
    buf[coff + 2..coff + 4].copy_from_slice(&5u16.to_le_bytes()); // NumberOfSections = 5
    buf[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes()); // SizeOfOptionalHeader = 240
    buf[coff + 18..coff + 20].copy_from_slice(&0x0022u16.to_le_bytes()); // EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

    // OptionalHeader.Magic (2 bytes at e_lfanew + 24)
    let opt = e_lfanew + 24;
    buf[opt..opt + 2].copy_from_slice(&0x020Bu16.to_le_bytes()); // PE32+ (64-bit)
    buf
}

#[test]
fn test_validate_mihomo_pe_binary_checks_pe64_coff_and_distinguishes_io() {
    let temp_dir = std::env::temp_dir().join(format!("clash_mini_pe_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    // 1. 不存在的文件应归类为 IoUnavailable，而非 Corrupted
    let missing_path = temp_dir.join("missing.exe");
    assert!(matches!(
        CoreUpdater::inspect_mihomo_pe_binary(&missing_path),
        CoreBinaryValidation::IoUnavailable(_)
    ));
    assert!(CoreUpdater::resolve_usable_custom_core(&temp_dir).is_none());

    // 2. 小于 5MB 的截断文件应判定为 Corrupted
    let small_path = temp_dir.join("small.exe");
    fs::write(&small_path, b"MZ\x00\x00").unwrap();
    assert!(matches!(
        CoreUpdater::inspect_mihomo_pe_binary(&small_path),
        CoreBinaryValidation::Corrupted(_)
    ));

    // 3. 满 5MB 且仅有 4 字节伪造 "MZ" + "PE\0\0"（无合法 AMD64 COFF / PE32+ Magic）必须被拦截为 Corrupted
    let fake_sig_only_path = temp_dir.join("fake_sig_only.exe");
    {
        let mut f = fs::File::create(&fake_sig_only_path).unwrap();
        let mut buf = vec![0u8; CoreUpdater::MIN_VALID_CORE_BYTES as usize];
        buf[0] = b'M';
        buf[1] = b'Z';
        buf[0x3C..0x40].copy_from_slice(&128u32.to_le_bytes());
        buf[128..132].copy_from_slice(b"PE\0\0");
        f.write_all(&buf).unwrap();
    }
    assert!(matches!(
        CoreUpdater::inspect_mihomo_pe_binary(&fake_sig_only_path),
        CoreBinaryValidation::Corrupted(_)
    ));
    assert!(CoreUpdater::validate_mihomo_pe_binary(&fake_sig_only_path).is_err());

    // 4. 满 5MB 但 COFF Machine 为 32 位 x86 (0x014c) 的错包文件必须被拦截为 Corrupted
    let x86_pe_path = temp_dir.join("x86_pe.exe");
    {
        let mut f = fs::File::create(&x86_pe_path).unwrap();
        let mut buf = build_minimal_valid_pe64_buffer(CoreUpdater::MIN_VALID_CORE_BYTES as usize);
        buf[132..134].copy_from_slice(&0x014Cu16.to_le_bytes());
        f.write_all(&buf).unwrap();
    }
    assert!(matches!(
        CoreUpdater::inspect_mihomo_pe_binary(&x86_pe_path),
        CoreBinaryValidation::Corrupted(_)
    ));

    // 5. 满 5MB 且具备完整合法 MZ + PE\0\0 + AMD64 COFF + PE32+ Magic + 节表边界的文件应通过校验
    let valid_pe_path = temp_dir.join("mini-mihomo.exe");
    {
        let mut f = fs::File::create(&valid_pe_path).unwrap();
        let buf = build_minimal_valid_pe64_buffer(CoreUpdater::MIN_VALID_CORE_BYTES as usize);
        f.write_all(&buf).unwrap();
    }
    assert_eq!(
        CoreUpdater::inspect_mihomo_pe_binary(&valid_pe_path),
        CoreBinaryValidation::Valid
    );
    assert!(CoreUpdater::validate_mihomo_pe_binary(&valid_pe_path).is_ok());
    assert_eq!(CoreUpdater::resolve_usable_custom_core(&temp_dir), Some(valid_pe_path));

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_parse_asset_sha256_digest_and_fail_closed_verification() {
    let valid_hex = "a".repeat(64);
    assert_eq!(
        CoreUpdater::parse_asset_sha256_digest(Some(&format!("SHA256:{}", "A".repeat(64)))),
        Some(valid_hex.clone())
    );
    assert_eq!(
        CoreUpdater::parse_asset_sha256_digest(Some(&valid_hex)),
        Some(valid_hex)
    );
    assert_eq!(CoreUpdater::parse_asset_sha256_digest(Some("sha256:short")), None);
    assert_eq!(CoreUpdater::parse_asset_sha256_digest(Some("md5:1234")), None);
    assert_eq!(CoreUpdater::parse_asset_sha256_digest(None), None);

    let temp_dir = std::env::temp_dir().join(format!("clash_mini_sha_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();
    let payload_path = temp_dir.join("core.zip");
    fs::write(&payload_path, b"clash-mini-test-payload").unwrap();

    let mut hasher = Sha256::new();
    hasher.update(b"clash-mini-test-payload");
    let expected = format!("sha256:{:x}", hasher.finalize());

    // 1. 缺失摘要必须拒绝 (Fail-Closed)
    assert!(CoreUpdater::verify_download_sha256(&payload_path, "core.zip", None).is_err());
    // 2. 不匹配摘要必须拒绝
    assert!(
        CoreUpdater::verify_download_sha256(&payload_path, "core.zip", Some(&format!("sha256:{}", "0".repeat(64))))
            .is_err()
    );
    // 3. 正确摘要通过校验
    assert!(CoreUpdater::verify_download_sha256(&payload_path, "core.zip", Some(&expected)).is_ok());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_validate_official_mihomo_asset_url_and_decompression_cap() {
    assert!(
        CoreUpdater::validate_official_mihomo_asset_url(
            "https://github.com/MetaCubeX/mihomo/releases/download/v1.19.34/mihomo-windows-amd64-v2-v1.19.34.zip"
        )
        .is_ok()
    );
    assert!(
        CoreUpdater::validate_official_mihomo_asset_url(
            "http://github.com/MetaCubeX/mihomo/releases/download/v1.19.34/a.zip"
        )
        .is_err()
    );
    assert!(
        CoreUpdater::validate_official_mihomo_asset_url(
            "https://evil.example/MetaCubeX/mihomo/releases/download/v1.19.34/a.zip"
        )
        .is_err()
    );
    assert!(
        CoreUpdater::validate_official_mihomo_asset_url("https://github.com/other/repo/releases/download/v1.0/a.zip")
            .is_err()
    );
    assert!(
        CoreUpdater::validate_official_mihomo_asset_url(
            "https://github.com/MetaCubeX/mihomo/releases/download/../other/a.zip"
        )
        .is_err()
    );

    let data = vec![b'x'; 100];
    let mut out_ok = Vec::new();
    assert_eq!(
        CoreUpdater::copy_with_max_bytes(io::Cursor::new(&data), &mut out_ok, 100).unwrap(),
        100
    );
    let mut out_err = Vec::new();
    assert!(CoreUpdater::copy_with_max_bytes(io::Cursor::new(&data), &mut out_err, 99).is_err());
}
