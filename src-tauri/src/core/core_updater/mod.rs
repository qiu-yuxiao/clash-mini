mod pe_verify;
#[cfg(test)]
mod tests;

use crate::{
    core::{handle::Handle, manager::CoreManager},
    utils::{
        dirs,
        network::{NetworkManager, ProxyType},
    },
};
use anyhow::{Context as _, Result, bail};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::fs;
use std::io;
use tauri::{AppHandle, Emitter as _};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GithubRelease {
    pub tag_name: String,
    pub assets: Vec<GithubAsset>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CoreUpgradeProgress {
    pub status: String, // "checking", "downloading", "extracting", "done", "error"
    pub progress: u32,  // percentage 0..100
    pub message: String,
}

pub struct CoreUpdater;

impl CoreUpdater {
    pub async fn check_latest_release() -> Result<GithubRelease> {
        let url = "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest";
        let nm = NetworkManager::new();
        let mut response = None;

        // 1. Try Localhost proxy
        if let Ok(client) = nm.create_request(ProxyType::Localhost, Some(10), None, false).await {
            if let Ok(resp) = client.get(url).send().await {
                if resp.status().is_success() {
                    response = Some(resp);
                }
            }
        }

        // 2. Try System proxy
        if response.is_none() {
            if let Ok(client) = nm.create_request(ProxyType::System, Some(10), None, false).await {
                if let Ok(resp) = client.get(url).send().await {
                    if resp.status().is_success() {
                        response = Some(resp);
                    }
                }
            }
        }

        // 3. Fallback to Direct connection
        let response = match response {
            Some(resp) => resp,
            None => {
                let client = nm
                    .create_request(ProxyType::None, Some(15), None, false)
                    .await
                    .context("failed to build reqwest client")?;
                client
                    .get(url)
                    .send()
                    .await
                    .context("failed to send request to GitHub API")?
            }
        };

        if !response.status().is_success() {
            bail!("GitHub API returned error: {}", response.status());
        }

        let release: GithubRelease = response
            .json()
            .await
            .context("failed to deserialize GitHub release JSON")?;

        Ok(release)
    }

    /// 在 release assets 中查找校验和文件并下载其内容。
    /// 返回值: Ok(Some(文件名 -> sha256 哈希))，未找到返回 Ok(None)。
    async fn fetch_checksums(
        client: &reqwest::Client,
        release: &GithubRelease,
    ) -> Result<Option<std::collections::HashMap<String, String>>> {
        // 常见的 GitHub Release 校验和文件命名
        const CHECKSUM_FILE_NAMES: &[&str] = &["checksums.txt", "sha256sums.txt", "sha256sum.txt"];

        let mut checksum_url: Option<String> = None;
        for asset in &release.assets {
            let lower = asset.name.to_lowercase();
            if CHECKSUM_FILE_NAMES.iter().any(|n| lower == *n) {
                checksum_url = Some(asset.browser_download_url.clone());
                break;
            }
        }

        let Some(url) = checksum_url else {
            return Ok(None);
        };

        Self::validate_official_mihomo_asset_url(&url)?;

        let response = client
            .get(&url)
            .send()
            .await
            .context("failed to download checksum file")?;

        if !response.status().is_success() {
            logging!(
                warn,
                Type::System,
                "Checksum file download failed with status: {}",
                response.status()
            );
            return Ok(None);
        }

        let body = response.text().await.context("failed to read checksum file body")?;

        let mut map = std::collections::HashMap::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // 常见格式: "<sha>  <filename>" 或 "<sha> *<filename>"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let sha = parts[0].to_lowercase();
            if sha.len() != 64 {
                continue;
            }
            let mut filename = parts[1].to_string();
            // 去掉二进制模式标记 '*'
            if let Some(stripped) = filename.strip_prefix('*') {
                filename = stripped.to_string();
            }
            // 解析 base path（保留完整文件名）
            if let Some(pos) = filename.rsplit('/').next() {
                map.insert(pos.to_lowercase(), sha);
            } else {
                map.insert(filename.to_lowercase(), sha);
            }
        }

        if map.is_empty() {
            return Ok(None);
        }

        Ok(Some(map))
    }

    /// 计算下载文件的 SHA256 并与官方摘要比对（Fail-Closed：缺失有效 SHA256 或不匹配一律拒绝升级，BUG-413）。
    fn verify_download_sha256(file_path: &std::path::Path, asset_name: &str, expected_sha: Option<&str>) -> Result<()> {
        let Some(expected_sha) = Self::parse_asset_sha256_digest(expected_sha) else {
            bail!("缺少官方 SHA-256 校验摘要 ({asset_name})，已拒绝未校验的内核升级");
        };

        let mut hasher = Sha256::new();
        let mut file = fs::File::open(file_path).context("failed to open downloaded file for hashing")?;
        io::copy(&mut file, &mut hasher).context("failed to hash downloaded file")?;
        let actual_sha = format!("{:x}", hasher.finalize());

        if actual_sha != expected_sha {
            bail!(
                "Integrity check failed for {}: expected {}, got {}",
                asset_name,
                expected_sha,
                actual_sha
            );
        }

        logging!(
            info,
            Type::System,
            "Integrity check passed for {} (sha256={})",
            asset_name,
            actual_sha
        );
        Ok(())
    }

    async fn download_body_to_file(
        client: &reqwest::Client,
        url: &str,
        app_handle: &AppHandle,
        asset_name: &str,
        dest_path: &std::path::Path,
        expected_size: Option<u64>,
    ) -> Result<()> {
        if let Some(sz) = expected_size
            && sz > Self::MAX_CORE_ARCHIVE_BYTES
        {
            bail!(
                "内核包声明体积超出安全上限 ({} bytes > {} bytes)",
                sz,
                Self::MAX_CORE_ARCHIVE_BYTES
            );
        }

        let mut response = client
            .get(url)
            .send()
            .await
            .context("failed to send request to GitHub")?;

        if !response.status().is_success() {
            bail!("HTTP status error: {}", response.status());
        }

        let content_length = response.content_length();
        if let Some(cl) = content_length
            && cl > Self::MAX_CORE_ARCHIVE_BYTES
        {
            bail!(
                "内核下载响应体体积超出安全上限 ({} bytes > {} bytes)",
                cl,
                Self::MAX_CORE_ARCHIVE_BYTES
            );
        }

        let total_size = content_length.or(expected_size).unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut dest_file = tokio::fs::File::create(dest_path)
            .await
            .context("failed to create temp download file")?;
        let mut last_emitted_percentage = 0u32;

        loop {
            let chunk_opt = match tokio::time::timeout(std::time::Duration::from_secs(20), response.chunk()).await {
                Ok(Ok(Some(chunk))) => Some(chunk),
                Ok(Ok(None)) => None,
                Ok(Err(e)) => {
                    bail!("读取网络数据流失败: {:?}", e);
                }
                Err(_) => {
                    bail!("读取网络数据超时 (stalled for 20s)");
                }
            };

            let Some(chunk) = chunk_opt else {
                break;
            };

            let next_downloaded = downloaded.saturating_add(chunk.len() as u64);
            if next_downloaded > Self::MAX_CORE_ARCHIVE_BYTES {
                bail!(
                    "内核下载实际接收体积超出安全上限 ({} bytes > {} bytes)",
                    next_downloaded,
                    Self::MAX_CORE_ARCHIVE_BYTES
                );
            }

            tokio::io::AsyncWriteExt::write_all(&mut dest_file, &chunk)
                .await
                .context("failed to write chunk to disk")?;
            downloaded = next_downloaded;
            if total_size > 0 {
                let percentage = ((downloaded as f64 / total_size as f64) * 80.0) as u32 + 10;
                if percentage != last_emitted_percentage {
                    last_emitted_percentage = percentage;
                    let payload = CoreUpgradeProgress {
                        status: "downloading".to_string(),
                        progress: percentage,
                        message: format!(
                            "正在下载 {}: {:.1} MB / {:.1} MB",
                            asset_name,
                            downloaded as f64 / 1_048_576.0,
                            total_size as f64 / 1_048_576.0
                        ),
                    };
                    let _ = app_handle.emit("core-upgrade-progress", payload);
                }
            }
        }

        tokio::io::AsyncWriteExt::flush(&mut dest_file)
            .await
            .context("failed to flush downloaded file")?;
        drop(dest_file);

        if let Some(sz) = expected_size
            && sz > 0
            && downloaded != sz
        {
            bail!("内核下载字节数与声明大小不符 (expected {sz} bytes, got {downloaded} bytes)");
        }

        Ok(())
    }

    pub async fn upgrade_core(app_handle: AppHandle, release: GithubRelease) -> Result<()> {
        let emit_progress = |status: &str, progress: u32, message: &str| {
            let payload = CoreUpgradeProgress {
                status: status.to_string(),
                progress,
                message: message.to_string(),
            };
            let _ = app_handle.emit("core-upgrade-progress", payload);
        };

        emit_progress("checking", 5, "正在检测适合您系统架构的内核包...");

        // 识别平台架构及匹配文件名
        // 仅支持 Windows x64（见平台支持范围），内核包名固定，无需再按平台分支
        let target_prefix = "mihomo-windows-amd64";
        let exact_gz = "mihomo-windows-amd64.exe.gz";

        // 在 Release assets 中寻找对应文件
        let mut target_asset = None;
        let exact_zip = format!("{}-{}.zip", target_prefix, release.tag_name).to_lowercase();
        let exact_gz = exact_gz.to_lowercase();

        for asset in &release.assets {
            let name = asset.name.to_lowercase();
            if name == exact_zip || name == exact_gz {
                target_asset = Some(asset);
                break;
            }
        }

        // 如果没有精准匹配，使用宽泛前缀匹配
        let asset = match target_asset {
            Some(a) => a,
            None => {
                let mut fallback_asset = None;
                for asset in &release.assets {
                    let name = asset.name.to_lowercase();
                    if name.starts_with(target_prefix) && (name.ends_with(".zip") || name.ends_with(".gz")) {
                        fallback_asset = Some(asset);
                        break;
                    }
                }
                match fallback_asset {
                    Some(a) => a,
                    None => {
                        let err_msg = "在 GitHub Release 中未找到适合您系统架构的内核包";
                        emit_progress("error", 0, err_msg);
                        bail!(err_msg);
                    }
                }
            }
        };

        let is_zip = asset.name.to_lowercase().ends_with(".zip");
        let download_url = asset.browser_download_url.clone();
        if let Err(e) = Self::validate_official_mihomo_asset_url(&download_url) {
            emit_progress("error", 0, &format!("下载地址校验失败: {e}"));
            return Err(e);
        }
        logging!(
            info,
            Type::System,
            "Core updater starting download from: {}",
            download_url
        );
        emit_progress("downloading", 10, &format!("开始下载: {}", asset.name));

        // Start downloading
        let nm = NetworkManager::new();
        let proxy_types = vec![ProxyType::Localhost, ProxyType::System, ProxyType::None];

        let app_dir = dirs::app_home_dir()?;
        let cores_dir = app_dir.join("cores");
        if !cores_dir.exists() {
            fs::create_dir_all(&cores_dir).context("failed to create cores directory")?;
        }
        let temp_download_path = cores_dir.join("mini-mihomo.download.tmp");
        let mut download_success = false;

        for proxy_type in proxy_types {
            logging!(
                info,
                Type::System,
                "Core updater trying download with proxy type: {:?}",
                proxy_type
            );
            if let Ok(client) = nm.create_request(proxy_type, Some(300), None, false).await {
                match Self::download_body_to_file(
                    &client,
                    &download_url,
                    &app_handle,
                    &asset.name,
                    &temp_download_path,
                    asset.size,
                )
                .await
                {
                    Ok(_) => {
                        logging!(
                            info,
                            Type::System,
                            "Core updater download succeeded using proxy type: {:?}",
                            proxy_type
                        );
                        download_success = true;
                        break;
                    }
                    Err(e) => {
                        logging!(
                            warn,
                            Type::System,
                            "Core updater download failed using proxy type {:?}: {:?}",
                            proxy_type,
                            e
                        );
                    }
                }
            }
        }

        if !download_success {
            let err_msg = "所有网络连接（代理/直连）均下载失败";
            emit_progress("error", 0, err_msg);
            if temp_download_path.exists() {
                let _ = fs::remove_file(&temp_download_path);
            }
            bail!(err_msg);
        }

        // 完整性校验：优先消费 GitHub Release API 原生携带的 asset.digest；缺失时回退拉取校验和文件，
        // 若仍无有效 SHA-256 摘要或比对失败则 Fail-Closed 中止升级（BUG-413）。
        emit_progress("verifying", 85, "正在校验下载文件完整性...");
        let mut expected_sha = Self::parse_asset_sha256_digest(asset.digest.as_deref());
        if expected_sha.is_none() {
            let nm_checksum = NetworkManager::new();
            let mut checksums_map: Option<std::collections::HashMap<String, String>> = None;
            for proxy_type in &[ProxyType::Localhost, ProxyType::System, ProxyType::None] {
                if let Ok(client) = nm_checksum.create_request(*proxy_type, Some(30), None, false).await {
                    match Self::fetch_checksums(&client, &release).await {
                        Ok(Some(map)) => {
                            checksums_map = Some(map);
                            break;
                        }
                        Ok(None) => break, // 仓库里就没这个文件，无需重试
                        Err(e) => {
                            logging!(
                                warn,
                                Type::System,
                                "fetch_checksums via {:?} failed: {:?}",
                                proxy_type,
                                e
                            );
                        }
                    }
                }
            }
            if let Some(map) = checksums_map {
                let lookup = asset.name.to_lowercase();
                expected_sha = map.get(&lookup).cloned();
            }
        }

        if let Err(e) = Self::verify_download_sha256(&temp_download_path, &asset.name, expected_sha.as_deref()) {
            let _ = fs::remove_file(&temp_download_path);
            emit_progress("error", 0, &format!("完整性校验失败: {:?}", e));
            return Err(e);
        }

        // Prepare destination path
        let custom_core_path = cores_dir.join("mini-mihomo.exe");

        logging!(info, Type::System, "Core updater stopping core to release file lock...");
        // Stop core — 若正在退出则跳过，关闭流程会自行停止 core
        if Handle::global().is_exiting() {
            return Ok(());
        }

        // 升级前记录合规目标选点（含持久化兜底），升级重启后统一校验并清理越界连接
        let saved_proxy_now = CoreManager::global().resolve_desired_proxy_selection().await;

        if let Err(e) = CoreManager::global().stop_core().await {
            logging!(error, Type::System, "Core updater: 停止 core 失败，中止升级: {}", e);
            return Err(anyhow::anyhow!("failed to stop core before upgrade: {}", e));
        }

        // 服务模式下 stop_core IPC 返回时 Session 0 中的 mini-mihomo.exe 可能尚未完全释放映像文件句柄；
        // 调用共享退出轮询（上限 3 秒，步进 50ms，join 异常保守视为仍存活），防止紧接着的 fs::rename 因共享冲突报错（BUG-346）。
        if !CoreManager::wait_for_mini_cores_exit(std::time::Duration::from_secs(3)).await {
            logging!(
                warn,
                Type::System,
                "Core updater: 等待旧 mini-mihomo 进程彻底退出超过 3s，继续进入 rename 短退避"
            );
        }

        emit_progress("extracting", 90, "正在解压并替换内核程序...");

        let temp_download_path_clone = temp_download_path.clone();
        let custom_core_path_clone = custom_core_path.clone();
        let staging_core_path = cores_dir.join("mini-mihomo.exe.staging");
        let staging_core_path_clone = staging_core_path.clone();
        let extract_res = tokio::task::spawn_blocking(move || -> Result<()> {
            // 【原子暂存替换防线 BUG-319 / BUG-346】
            // 严禁直接对正式路径 File::create（会在解压前将既有内核截断为 0 字节）；
            // 统一先解压至 .staging 临时文件，校验体积与 DOS/PE 签名头通过后再覆盖。
            let _ = fs::remove_file(&staging_core_path_clone);
            let write_result = (|| -> Result<()> {
                let mut dest_file =
                    fs::File::create(&staging_core_path_clone).context("failed to create staging core file")?;
                if is_zip {
                    let archive_file =
                        fs::File::open(&temp_download_path_clone).context("failed to open downloaded zip archive")?;
                    let mut archive = zip::ZipArchive::new(archive_file).context("failed to parse zip archive")?;
                    let mut mihomo_file_idx = None;
                    for i in 0..archive.len() {
                        let file = archive.by_index(i)?;
                        let name = file.name().to_lowercase();
                        if name.contains("mihomo") && (name.ends_with(".exe") || !name.contains('.')) {
                            mihomo_file_idx = Some(i);
                            break;
                        }
                    }
                    let idx = match mihomo_file_idx {
                        Some(i) => i,
                        None => {
                            let _ = fs::remove_file(&temp_download_path_clone);
                            bail!("在 ZIP 压缩包内未找到 mini-mihomo 程序二进制");
                        }
                    };
                    let file = archive.by_index(idx)?;
                    Self::copy_with_max_bytes(file, &mut dest_file, Self::MAX_CORE_EXTRACTED_BYTES)?;
                } else {
                    let archive_file =
                        fs::File::open(&temp_download_path_clone).context("failed to open downloaded gz archive")?;
                    let decoder = flate2::read::GzDecoder::new(archive_file);
                    Self::copy_with_max_bytes(decoder, &mut dest_file, Self::MAX_CORE_EXTRACTED_BYTES)?;
                }
                dest_file.sync_all().context("failed to sync staging core file")?;
                Ok(())
            })();

            if let Err(err) = write_result {
                let _ = fs::remove_file(&staging_core_path_clone);
                return Err(err);
            }

            if let Err(err) = Self::validate_mihomo_pe_binary(&staging_core_path_clone) {
                let _ = fs::remove_file(&staging_core_path_clone);
                return Err(err).context("解压后的内核二进制未通过 PE 结构校验，已中止替换保留原内核");
            }

            // Windows 下杀毒软件扫描或 Session 0 映像句柄释放可能存在百毫秒级尾延迟，遇 PermissionDenied 短退避重试
            let mut last_rename_err = None;
            for attempt in 0..10 {
                match fs::rename(&staging_core_path_clone, &custom_core_path_clone) {
                    Ok(()) => {
                        last_rename_err = None;
                        break;
                    }
                    Err(err) if err.kind() == io::ErrorKind::PermissionDenied && attempt < 9 => {
                        last_rename_err = Some(err);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(err) => {
                        last_rename_err = Some(err);
                        break;
                    }
                }
            }
            if let Some(rename_err) = last_rename_err {
                let _ = fs::remove_file(&staging_core_path_clone);
                return Err(rename_err).context("failed to atomically replace custom core binary");
            }
            Ok(())
        })
        .await;

        let extract_res = match extract_res {
            Ok(res) => res,
            Err(join_err) => Err(anyhow::anyhow!("spawn_blocking panicked: {:?}", join_err)),
        };

        // Clean up temp file
        if temp_download_path.exists() {
            let _ = fs::remove_file(&temp_download_path);
        }

        if let Err(e) = extract_res {
            if !Handle::global().is_exiting() {
                if let Err(e2) = CoreManager::global().start_core().await {
                    logging!(warn, Type::System, "Core updater: 恢复性启动 core 失败: {}", e2);
                }
                CoreManager::global().spawn_ensure_proxy_group_selection(saved_proxy_now.clone());
            }
            emit_progress("error", 0, &format!("解压替换失败: {:?}", e));
            return Err(e);
        }

        logging!(
            info,
            Type::System,
            "Core updater successfully updated core binary. Restarting core..."
        );

        // Start core — 仅在未退出时执行，退出流程会自行处理
        if !Handle::global().is_exiting() {
            if let Err(e) = CoreManager::global().start_core().await {
                logging!(error, Type::System, "Core updater: 升级后重启 core 失败: {}", e);
                // 二进制已成功替换但 core 启动失败，向前端报告真实错误而非假成功
                emit_progress(
                    "error",
                    0,
                    &format!("内核二进制已更新，但重启失败: {}。请手动重启 Clash Mini。", e),
                );
                return Err(anyhow::anyhow!("start_core failed after upgrade: {}", e));
            }
            CoreManager::global().spawn_ensure_proxy_group_selection(saved_proxy_now);
        }

        emit_progress("done", 100, "内核更新成功！核心已成功重启。");
        Ok(())
    }
}
