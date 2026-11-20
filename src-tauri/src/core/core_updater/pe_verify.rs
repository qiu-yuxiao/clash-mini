use super::CoreUpdater;
use anyhow::{Context as _, Result, bail};
use clash_verge_logging::{Type, logging};
use std::fs;
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBinaryValidation {
    /// 体积、DOS 头、COFF 头（AMD64 可执行映像）、PE32+ Optional Header 与节表边界均合法
    Valid,
    /// 文件确系截断或 PE 结构损坏（可安全清理该托管热更产物）
    Corrupted(String),
    /// 瞬时 I/O 不可读（如杀软扫描独占、句柄未释放、权限暂不可达），严禁误删原文件
    IoUnavailable(String),
}

impl CoreUpdater {
    pub const MIN_VALID_CORE_BYTES: u64 = 5 * 1024 * 1024;
    pub const MAX_CORE_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
    pub const MAX_CORE_EXTRACTED_BYTES: u64 = 150 * 1024 * 1024;
    const OFFICIAL_MIHOMO_RELEASE_DOWNLOAD_PREFIX: &'static str = "/MetaCubeX/mihomo/releases/download/";
    const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
    const IMAGE_FILE_EXECUTABLE_IMAGE: u16 = 0x0002;
    const IMAGE_NT_OPTIONAL_HDR64_MAGIC: u16 = 0x020B;

    /// 从 GitHub Release Asset `digest`（如 `"sha256:<64 hex>"`）或裸 64 位 SHA-256 字符串解析标准小写摘要（BUG-413）。
    pub fn parse_asset_sha256_digest(digest: Option<&str>) -> Option<String> {
        let raw = digest?.trim();
        let hex_part = if raw.len() > 7 && raw[..7].eq_ignore_ascii_case("sha256:") {
            raw[7..].trim()
        } else {
            raw
        };
        if hex_part.len() == 64 && hex_part.bytes().all(|b| b.is_ascii_hexdigit()) {
            Some(hex_part.to_ascii_lowercase())
        } else {
            None
        }
    }

    /// 锁定内核下载与校验和地址必须源自官方 GitHub 仓库 Release 下载路径（BUG-413）。
    pub fn validate_official_mihomo_asset_url(raw_url: &str) -> Result<()> {
        let parsed = url::Url::parse(raw_url).context("无效的内核下载 URL")?;
        if parsed.scheme() != "https" {
            bail!("内核下载仅允许 HTTPS 协议: {raw_url}");
        }
        if parsed.host_str() != Some("github.com") {
            bail!("内核下载来源域名非法（仅允许 github.com）: {raw_url}");
        }
        if parsed.port().is_some() || !parsed.username().is_empty() || parsed.password().is_some() {
            bail!("内核下载 URL 禁止包含非标准端口或用户信息: {raw_url}");
        }
        let path = parsed.path();
        if !path.starts_with(Self::OFFICIAL_MIHOMO_RELEASE_DOWNLOAD_PREFIX) {
            bail!("内核下载路径不在官方 MetaCubeX/mihomo Release 目录内: {raw_url}");
        }
        if path.split('/').any(|seg| seg == "..") {
            bail!("内核下载 URL 路径非法: {raw_url}");
        }
        Ok(())
    }

    /// 带上限的流式复制，防解压炸弹写满磁盘（BUG-413）。
    pub(super) fn copy_with_max_bytes<R: io::Read, W: io::Write>(
        reader: R,
        writer: &mut W,
        max_bytes: u64,
    ) -> Result<u64> {
        let mut limited = io::Read::take(reader, max_bytes.saturating_add(1));
        let copied = io::copy(&mut limited, writer).context("failed to decompress core binary")?;
        if copied > max_bytes {
            bail!(
                "解压后的内核文件体积超过安全上限 ({} bytes > {} bytes)，已中止",
                copied,
                max_bytes
            );
        }
        Ok(copied)
    }

    /// 深度检查 Mihomo Windows x64 PE32+ 可执行文件，严格区分「真实结构损坏」与「瞬时 I/O 不可读」（BUG-319 / BUG-346）。
    pub fn inspect_mihomo_pe_binary(path: &std::path::Path) -> CoreBinaryValidation {
        use io::{Read as _, Seek as _};

        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return CoreBinaryValidation::IoUnavailable(format!("读取内核文件元数据失败: {e}")),
        };
        if !meta.is_file() {
            return CoreBinaryValidation::Corrupted("目标内核路径不是普通文件".to_string());
        }
        let file_len = meta.len();
        if file_len < Self::MIN_VALID_CORE_BYTES {
            return CoreBinaryValidation::Corrupted(format!(
                "内核二进制体积异常（{} bytes < 5MB），判定为截断或损坏文件",
                file_len
            ));
        }

        let mut file = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return CoreBinaryValidation::IoUnavailable(format!(
                    "打开内核文件失败（可能正被安全软件扫描或其它进程占用）: {e}"
                ));
            }
        };

        let mut dos_header = [0u8; 64];
        if let Err(e) = file.read_exact(&mut dos_header) {
            return if e.kind() == io::ErrorKind::UnexpectedEof {
                CoreBinaryValidation::Corrupted(format!("读取 64 字节 DOS 头意外截断: {e}"))
            } else {
                CoreBinaryValidation::IoUnavailable(format!("读取 DOS 头 I/O 异常: {e}"))
            };
        }
        if &dos_header[0..2] != b"MZ" {
            return CoreBinaryValidation::Corrupted("内核文件缺少有效 DOS MZ 签名头".to_string());
        }

        let e_lfanew =
            u32::from_le_bytes([dos_header[0x3C], dos_header[0x3D], dos_header[0x3E], dos_header[0x3F]]) as u64;
        // 至少容纳 PE\0\0 (4B) + COFF FileHeader (20B) + OptionalHeader.Magic (2B) = 26 字节
        if e_lfanew < 64 || e_lfanew.saturating_add(26) > file_len {
            return CoreBinaryValidation::Corrupted(format!(
                "内核文件 PE 头偏移指针越界损坏 (e_lfanew={}, len={})",
                e_lfanew, file_len
            ));
        }

        if let Err(e) = file.seek(io::SeekFrom::Start(e_lfanew)) {
            return CoreBinaryValidation::IoUnavailable(format!("定位 PE 头偏移 I/O 失败: {e}"));
        }

        let mut nt_prefix = [0u8; 26];
        if let Err(e) = file.read_exact(&mut nt_prefix) {
            return if e.kind() == io::ErrorKind::UnexpectedEof {
                CoreBinaryValidation::Corrupted(format!("读取 PE/COFF 头意外截断: {e}"))
            } else {
                CoreBinaryValidation::IoUnavailable(format!("读取 PE/COFF 头 I/O 异常: {e}"))
            };
        }

        if &nt_prefix[0..4] != b"PE\0\0" {
            return CoreBinaryValidation::Corrupted("内核文件缺少有效 Windows PE\\0\\0 签名头".to_string());
        }

        let machine = u16::from_le_bytes([nt_prefix[4], nt_prefix[5]]);
        if machine != Self::IMAGE_FILE_MACHINE_AMD64 {
            return CoreBinaryValidation::Corrupted(format!(
                "内核 COFF Machine 架构不匹配 (expected=0x{:04X}, actual=0x{:04X})",
                Self::IMAGE_FILE_MACHINE_AMD64,
                machine
            ));
        }

        let num_sections = u16::from_le_bytes([nt_prefix[6], nt_prefix[7]]);
        if !(1..=96).contains(&num_sections) {
            return CoreBinaryValidation::Corrupted(format!("内核 COFF 节数量异常 (NumberOfSections={num_sections})"));
        }

        let size_of_opt_hdr = u16::from_le_bytes([nt_prefix[20], nt_prefix[21]]);
        if size_of_opt_hdr < 112 {
            return CoreBinaryValidation::Corrupted(format!(
                "内核 PE32+ Optional Header 长度过小 (SizeOfOptionalHeader={size_of_opt_hdr})"
            ));
        }

        let characteristics = u16::from_le_bytes([nt_prefix[22], nt_prefix[23]]);
        if (characteristics & Self::IMAGE_FILE_EXECUTABLE_IMAGE) == 0 {
            return CoreBinaryValidation::Corrupted(format!(
                "内核 COFF Characteristics 未标记为可执行映像 (0x{characteristics:04X})"
            ));
        }

        let opt_magic = u16::from_le_bytes([nt_prefix[24], nt_prefix[25]]);
        if opt_magic != Self::IMAGE_NT_OPTIONAL_HDR64_MAGIC {
            return CoreBinaryValidation::Corrupted(format!(
                "内核 Optional Header Magic 非 64 位 PE32+ (expected=0x{:04X}, actual=0x{:04X})",
                Self::IMAGE_NT_OPTIONAL_HDR64_MAGIC,
                opt_magic
            ));
        }

        let section_table_end = e_lfanew
            .saturating_add(24)
            .saturating_add(size_of_opt_hdr as u64)
            .saturating_add((num_sections as u64).saturating_mul(40));
        if section_table_end > file_len {
            return CoreBinaryValidation::Corrupted(format!(
                "内核 PE 节表越出文件末尾 (section_table_end={section_table_end}, len={file_len})"
            ));
        }

        CoreBinaryValidation::Valid
    }

    /// 供解压暂存区 `.staging` 等必须通过完整校验的调用点使用。
    pub fn validate_mihomo_pe_binary(path: &std::path::Path) -> Result<()> {
        match Self::inspect_mihomo_pe_binary(path) {
            CoreBinaryValidation::Valid => Ok(()),
            CoreBinaryValidation::Corrupted(reason) => bail!("{reason}"),
            CoreBinaryValidation::IoUnavailable(reason) => bail!("内核文件 I/O 校验失败: {reason}"),
        }
    }

    /// 统一解析托管热升级目录 `cores/mini-mihomo.exe`（供 Sidecar / Service / Validate 三处消费方共用）：
    /// - 文件不存在：静默返回 `None`（使用内置内核）；
    /// - 校验通过（`Valid`）：返回可用的自定义内核路径；
    /// - 瞬时 I/O 不可读（`IoUnavailable`，如杀软扫描锁定）：记录告警、**保留原文件不删除**，本次临时回退内置内核；
    /// - 确系截断或 PE 结构损坏（`Corrupted`）：记录告警、清理损坏产物（若删除失败同样记录告警），回退内置内核。
    pub fn resolve_usable_custom_core(cores_dir: &std::path::Path) -> Option<std::path::PathBuf> {
        let custom_core_path = cores_dir.join("mini-mihomo.exe");
        match fs::symlink_metadata(&custom_core_path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
            Err(e) => {
                logging!(
                    warn,
                    Type::Core,
                    "读取托管热更内核元数据失败 ({e})，保留原文件并仅在本次回退内置内核: {}",
                    custom_core_path.display()
                );
                return None;
            }
            Ok(_) => {}
        }

        match Self::inspect_mihomo_pe_binary(&custom_core_path) {
            CoreBinaryValidation::Valid => Some(custom_core_path),
            CoreBinaryValidation::IoUnavailable(reason) => {
                logging!(
                    warn,
                    Type::Core,
                    "托管热更内核当前暂时不可读 ({reason})，保留原文件不删除，本次临时回退内置内核: {}",
                    custom_core_path.display()
                );
                None
            }
            CoreBinaryValidation::Corrupted(reason) => {
                logging!(
                    warn,
                    Type::Core,
                    "检测到结构损坏的托管热更内核 ({reason})，自动清理并回退内置内核: {}",
                    custom_core_path.display()
                );
                if let Err(rm_err) = fs::remove_file(&custom_core_path) {
                    logging!(
                        warn,
                        Type::Core,
                        "清理损坏的托管热更内核失败 ({}): {rm_err}",
                        custom_core_path.display()
                    );
                }
                None
            }
        }
    }
}
