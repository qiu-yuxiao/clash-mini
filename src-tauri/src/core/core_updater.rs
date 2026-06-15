use crate::{
    core::manager::CoreManager,
    utils::{dirs, network::{NetworkManager, ProxyType}},
};
use anyhow::{Result, Context, bail};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
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
                let client = nm.create_request(ProxyType::None, Some(15), None, false)
                    .await
                    .context("failed to build reqwest client")?;
                client.get(url)
                    .send()
                    .await
                    .context("failed to send request to GitHub API")?
            }
        };

        if !response.status().is_success() {
            bail!("GitHub API returned error: {}", response.status());
        }

        let release: GithubRelease = response.json()
            .await
            .context("failed to deserialize GitHub release JSON")?;

        Ok(release)
    }

    async fn download_body(
        client: &reqwest::Client,
        url: &str,
        app_handle: &AppHandle,
        asset_name: &str,
    ) -> Result<Vec<u8>> {
        let mut response = client.get(url)
            .send()
            .await
            .context("failed to send request to GitHub")?;

        if !response.status().is_success() {
            bail!("HTTP status error: {}", response.status());
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut bytes = Vec::new();

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

            bytes.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            if total_size > 0 {
                let percentage = ((downloaded as f64 / total_size as f64) * 80.0) as u32 + 10;
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

        Ok(bytes)
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

        // Detect OS and Arch
        let target_os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else {
            "linux"
        };

        let target_arch = if cfg!(target_arch = "x86_64") {
            "amd64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else if cfg!(target_arch = "x86") {
            "386"
        } else {
            "amd64" // fallback
        };

        let target_prefix = format!("mihomo-{}-{}", target_os, target_arch);
        logging!(info, Type::System, "Core updater searching for asset prefix: {}", target_prefix);

        let matched_asset = {
            let assets = release.assets.clone();
            let exact_zip = format!("{}-{}.zip", target_prefix, release.tag_name).to_lowercase();
            let exact_gz = format!("{}-{}.gz", target_prefix, release.tag_name).to_lowercase();
            
            assets.iter().find(|asset| {
                let name = asset.name.to_lowercase();
                name == exact_zip || name == exact_gz
            })
            .cloned()
            .or_else(|| {
                assets.into_iter().find(|asset| {
                    let name = asset.name.to_lowercase();
                    name.contains(&target_prefix) && !name.contains("compat") && (name.ends_with(".zip") || name.ends_with(".gz"))
                })
            })
        };

        let asset = match matched_asset {
            Some(a) => a,
            None => {
                let err_msg = format!("未找到适用于 {}-{} 的内核安装包", target_os, target_arch);
                emit_progress("error", 0, &err_msg);
                bail!(err_msg);
            }
        };

        let is_zip = asset.name.to_lowercase().ends_with(".zip");
        let download_url = asset.browser_download_url.clone();
        logging!(info, Type::System, "Core updater starting download from: {}", download_url);
        emit_progress("downloading", 10, &format!("开始下载: {}", asset.name));

        // Start downloading
        let nm = NetworkManager::new();
        let proxy_types = vec![ProxyType::Localhost, ProxyType::System, ProxyType::None];
        let mut downloaded_bytes = None;

        for proxy_type in proxy_types {
            logging!(info, Type::System, "Core updater trying download with proxy type: {:?}", proxy_type);
            if let Ok(client) = nm.create_request(proxy_type, Some(300), None, false).await {
                match Self::download_body(&client, &download_url, &app_handle, &asset.name).await {
                    Ok(b) => {
                        logging!(info, Type::System, "Core updater download succeeded using proxy type: {:?}", proxy_type);
                        downloaded_bytes = Some(b);
                        break;
                    }
                    Err(e) => {
                        logging!(warn, Type::System, "Core updater download failed using proxy type {:?}: {:?}", proxy_type, e);
                    }
                }
            }
        }

        let decompressed_bytes = match downloaded_bytes {
            Some(b) => {
                emit_progress("extracting", 90, "正在解压并替换内核程序...");
                if is_zip {
                    let reader = io::Cursor::new(b);
                    let mut archive = zip::ZipArchive::new(reader).context("failed to parse zip archive")?;
                    let mut mihomo_file_idx = None;
                    for i in 0..archive.len() {
                        let file = archive.by_index(i)?;
                        let name = file.name().to_lowercase();
                        if name.contains("mihomo") && (name.ends_with(".exe") || !name.contains(".")) {
                            mihomo_file_idx = Some(i);
                            break;
                        }
                    }
                    let idx = match mihomo_file_idx {
                        Some(i) => i,
                        None => {
                            let err_msg = "在 ZIP 压缩包内未找到 mini-mihomo 程序二进制";
                            emit_progress("error", 0, err_msg);
                            bail!(err_msg);
                        }
                    };
                    let mut file = archive.by_index(idx)?;
                    let mut buf = Vec::new();
                    io::copy(&mut file, &mut buf).context("failed to extract file from zip")?;
                    buf
                } else {
                    let mut decoder = flate2::read::GzDecoder::new(io::Cursor::new(b));
                    let mut buf = Vec::new();
                    io::copy(&mut decoder, &mut buf).context("failed to decompress gzip archive")?;
                    buf
                }
            }
            None => {
                let err_msg = "所有网络连接（代理/直连）均下载失败";
                emit_progress("error", 0, err_msg);
                bail!(err_msg);
            }
        };

        // Prepare destination path
        let app_dir = dirs::app_home_dir()?;
        let cores_dir = app_dir.join("cores");
        if !cores_dir.exists() {
            fs::create_dir_all(&cores_dir).context("failed to create cores directory")?;
        }

        let core_name = if cfg!(windows) { "mini-mihomo.exe" } else { "mini-mihomo" };
        let custom_core_path = cores_dir.join(core_name);

        logging!(info, Type::System, "Core updater stopping core to release file lock...");
        // Stop core
        let _ = CoreManager::global().stop_core().await;

        // Write to destination
        fs::write(&custom_core_path, decompressed_bytes).context("failed to write core binary to disk")?;

        // Set Unix execute permission
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&custom_core_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&custom_core_path, perms).context("failed to set execution permission on unix")?;
        }

        logging!(info, Type::System, "Core updater successfully updated core binary. Restarting core...");
        
        // Start core
        let _ = CoreManager::global().start_core().await;

        emit_progress("done", 100, "内核更新成功！核心已成功重启。");
        Ok(())
    }
}
