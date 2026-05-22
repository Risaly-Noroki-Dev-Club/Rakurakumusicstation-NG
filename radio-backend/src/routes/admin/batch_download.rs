use crate::app::state::AppState;
use crate::error::AppError;
use crate::models::{
    ApiResponse, BatchDownloadRequest, BatchDownloadResponse, BatchDownloadResultItem,
    BatchDownloadStatus, DownloadEvent,
};
use crate::routes::admin::get_admin;
use crate::services::download_tasks::{
    ext_from_type, generate_task_id, insert_task, quality_to_ncm_level, remove_task,
    sanitize_filename, subscribe_task, task_snapshot, BatchTask,
};
use crate::services::ncm::{api, NcmClient};
use crate::services::netdisk;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::sse::{Event, Sse},
    Json,
};
use futures_util::stream::{unfold, Stream};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;

/// POST /api/admin/download/batch — 启动批量下载任务
pub async fn start_batch_download(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BatchDownloadRequest>,
) -> Result<Json<ApiResponse<BatchDownloadResponse>>, AppError> {
    let _admin = get_admin(&state, &headers).await?;

    if body.items.is_empty() {
        return Err(AppError::BadRequest("下载列表不能为空".into()));
    }

    let source = body.source.clone();
    let task_id = generate_task_id();
    let total = body.items.len();

    let task = BatchTask::new(source.clone(), total);

    insert_task(task_id.clone(), task.clone());

    let media_path = state.config.audio_engine.media_path.clone();
    let player_handle = state.player_handle.clone();

    match source.as_str() {
        "ncm" => {
            let device_id = if state.config.ncm.device_id.is_empty() {
                None
            } else {
                Some(state.config.ncm.device_id.clone())
            };
            let ncm_cookie = crate::routes::admin::ncm::read_admin_ncm_cookie();
            let client = NcmClient::new(device_id, ncm_cookie);
            let quality = body.quality.unwrap_or_else(|| "exhigh".into());
            let lyrics_mode = if body.lyrics_save_mode.is_empty() {
                "separate".to_string()
            } else {
                body.lyrics_save_mode.clone()
            };

            tokio::spawn(async move {
                run_ncm_batch(task, client, body.items, quality, lyrics_mode, media_path).await;
                player_handle.send_command(radio_engine::types::AudioCommand {
                    cmd_type: radio_engine::types::AudioCommandType::ReloadQueue,
                    song_id: None,
                    file_path: None,
                });
            });
        }
        "netdisk" => {
            tokio::spawn(async move {
                run_netdisk_batch(task, body.items, media_path).await;
                player_handle.send_command(radio_engine::types::AudioCommand {
                    cmd_type: radio_engine::types::AudioCommandType::ReloadQueue,
                    song_id: None,
                    file_path: None,
                });
            });
        }
        "spotify" => {
            remove_task(&task_id);
            return Err(AppError::BadRequest("Spotify 下载尚未实现".into()));
        }
        _ => {
            remove_task(&task_id);
            return Err(AppError::BadRequest(format!("不支持的下载源: {}", source)));
        }
    }

    Ok(Json(ApiResponse::ok(BatchDownloadResponse {
        task_id,
        total,
    })))
}

#[derive(Debug, Deserialize)]
pub struct BatchStreamQuery {
    pub task_id: String,
}

/// GET /api/admin/download/batch/stream — SSE 批量下载进度（按 task_id）
pub async fn batch_download_stream(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<BatchStreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let rx = subscribe_task(&query.task_id)
        .ok_or_else(|| AppError::BadRequest("任务不存在或已结束".into()))?;

    let stream = unfold(rx, |mut rx| async {
        match rx.recv().await {
            Ok(ev) => {
                let data = serde_json::to_string(&ev).unwrap_or_default();
                Some((
                    Ok::<_, std::convert::Infallible>(Event::default().data(data)),
                    rx,
                ))
            }
            Err(broadcast::error::RecvError::Lagged(_)) => Some((
                Ok::<_, std::convert::Infallible>(
                    Event::default().data(
                        serde_json::to_string(&DownloadEvent {
                            log: "...".to_string(),
                            done: false,
                            task_id: None,
                        })
                        .unwrap_or_default(),
                    ),
                ),
                rx,
            )),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });

    Ok(Sse::new(stream))
}

/// GET /api/admin/download/batch/status — 获取批量下载状态
pub async fn batch_download_status(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<BatchStreamQuery>,
) -> Result<Json<ApiResponse<BatchDownloadStatus>>, AppError> {
    let _admin = get_admin(&_state, &headers).await?;

    let snapshot = task_snapshot(&query.task_id)
        .ok_or_else(|| AppError::BadRequest("任务不存在或已结束".into()))?;

    Ok(Json(ApiResponse::ok(BatchDownloadStatus {
        task_id: query.task_id,
        running: snapshot.running,
        source: snapshot.source,
        total: snapshot.total,
        success: snapshot.success,
        failed: snapshot.failed,
        items: snapshot.items,
    })))
}

// ─── NCM 批量下载 ───────────────────────────────────────────────

async fn run_ncm_batch(
    task: BatchTask,
    client: NcmClient,
    items: Vec<crate::models::BatchDownloadItem>,
    quality: String,
    lyrics_mode: String,
    media_path: String,
) {
    let client = Arc::new(client);
    let total = items.len();

    let _ = task.tx.send(DownloadEvent {
        log: format!("🎵 NCM 批量下载开始，共 {} 首", total),
        done: false,
        task_id: None,
    });

    for (i, item) in items.iter().enumerate() {
        let keyword = if let Some(ref title) = item.title {
            if let Some(ref artist) = item.artist {
                format!("{} {}", artist, title)
            } else {
                title.clone()
            }
        } else {
            item.id.clone().unwrap_or_default()
        };

        if keyword.is_empty() {
            let result = BatchDownloadResultItem {
                id: item.id.clone(),
                title: item.title.clone(),
                artist: item.artist.clone(),
                success: false,
                error: Some("缺少关键词".into()),
                file_path: None,
            };
            task.items
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(result.clone());
            task.failed
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let _ = task.tx.send(DownloadEvent {
                log: format!("❌ [{}/{}] 缺少关键词", i + 1, total),
                done: false,
                task_id: None,
            });
            continue;
        }

        let _ = task.tx.send(DownloadEvent {
            log: format!("🔍 [{}/{}] 搜索: {}", i + 1, total, keyword),
            done: false,
            task_id: None,
        });

        match ncm_download_one(
            &client,
            item,
            &quality,
            &lyrics_mode,
            &media_path,
            &task,
            i,
            total,
        )
        .await
        {
            Ok(path) => {
                let result = BatchDownloadResultItem {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    artist: item.artist.clone(),
                    success: true,
                    error: None,
                    file_path: Some(path),
                };
                task.items
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(result);
                task.success
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Err(e) => {
                let result = BatchDownloadResultItem {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    artist: item.artist.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    file_path: None,
                };
                task.items
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(result);
                task.failed
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = task.tx.send(DownloadEvent {
                    log: format!("❌ [{}/{}] 失败: {}", i + 1, total, e),
                    done: false,
                    task_id: None,
                });
            }
        }
    }

    let success = task.success.load(std::sync::atomic::Ordering::SeqCst);
    let failed = task.failed.load(std::sync::atomic::Ordering::SeqCst);
    let _ = task.tx.send(DownloadEvent {
        log: format!("🎉 NCM 批量下载完成! 成功: {}, 失败: {}", success, failed),
        done: true,
        task_id: None,
    });

    task.running
        .store(false, std::sync::atomic::Ordering::SeqCst);
}

async fn ncm_download_one(
    client: &NcmClient,
    item: &crate::models::BatchDownloadItem,
    quality: &str,
    lyrics_mode: &str,
    media_path: &str,
    task: &BatchTask,
    idx: usize,
    total: usize,
) -> anyhow::Result<String> {
    let keyword = if let Some(ref title) = item.title {
        if let Some(ref artist) = item.artist {
            format!("{} {}", artist, title)
        } else {
            title.clone()
        }
    } else {
        item.id.clone().unwrap_or_default()
    };

    // 1. Search
    let results = api::search_song(client, &keyword, 5).await?;
    if results.is_empty() {
        anyhow::bail!("未找到歌曲");
    }

    let song = &results[0];
    let artist_name = song
        .artists
        .first()
        .map(|a| a.name.as_str())
        .unwrap_or("")
        .to_string();

    let _ = task.tx.send(DownloadEvent {
        log: format!(
            "✅ [{}/{}] 找到: {} - {} (ID: {})",
            idx + 1,
            total,
            artist_name,
            song.name,
            song.id
        ),
        done: false,
        task_id: None,
    });

    // 2. Get download URL
    let level = quality_to_ncm_level(quality);
    let urls = api::get_song_url(client, &[song.id], level).await?;
    if urls.is_empty() || urls[0].url.is_empty() {
        anyhow::bail!("无法获取下载链接");
    }

    let url_data = &urls[0];
    let ext = ext_from_type(&url_data.file_type, &url_data.url);

    // 3. Download file
    let safe_artist = sanitize_filename(&artist_name);
    let safe_title = sanitize_filename(&song.name);
    let filename = if let Some(ref save_as) = item.save_as {
        if save_as.contains('.') {
            sanitize_filename(save_as)
        } else {
            format!("{}.{}", sanitize_filename(save_as), ext)
        }
    } else {
        format!("{} - {}.{}", safe_artist, safe_title, ext)
    };

    let output_dir = PathBuf::from(media_path).join("downloads");
    tokio::fs::create_dir_all(&output_dir).await.ok();
    let filepath = output_dir.join(&filename);

    let _ = task.tx.send(DownloadEvent {
        log: format!(
            "⬇️ [{}/{}] 下载: {} ({} bytes)",
            idx + 1,
            total,
            filename,
            url_data.size
        ),
        done: false,
        task_id: None,
    });

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let resp = http.get(&url_data.url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("音频文件下载失败: HTTP {}", status);
    }
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("音频文件下载失败: 返回空文件");
    }

    // MD5 check
    if !url_data.md5.is_empty() {
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&bytes);
        let file_md5 = format!("{:x}", hasher.finalize());
        if file_md5 != url_data.md5 {
            let _ = task.tx.send(DownloadEvent {
                log: format!(
                    "⚠️ [{}/{}] MD5 校验失败 (期望 {}, 实际 {})",
                    idx + 1,
                    total,
                    url_data.md5,
                    file_md5
                ),
                done: false,
                task_id: None,
            });
        } else {
            let _ = task.tx.send(DownloadEvent {
                log: format!("✅ [{}/{}] MD5 校验通过", idx + 1, total),
                done: false,
                task_id: None,
            });
        }
    }

    tokio::fs::write(&filepath, &bytes).await?;
    let _ = task.tx.send(DownloadEvent {
        log: format!("✅ [{}/{}] 已保存: {}", idx + 1, total, filename),
        done: false,
        task_id: None,
    });

    // 4. Download lyrics (unless override_lyrics is true)
    if !item.override_lyrics && lyrics_mode != "none" {
        match api::get_song_lyric(client, song.id).await {
            Ok(Some(lyric)) if !lyric.is_empty() => {
                if lyrics_mode == "overwrite" {
                    // Save as .lrc with same name as audio file
                    let lrc_path = filepath.with_extension("lrc");
                    if let Err(e) = tokio::fs::write(&lrc_path, lyric).await {
                        let _ = task.tx.send(DownloadEvent {
                            log: format!("⚠️ 歌词保存失败: {}", e),
                            done: false,
                            task_id: None,
                        });
                    } else {
                        let _ = task.tx.send(DownloadEvent {
                            log: format!("📝 歌词已保存: {}", lrc_path.display()),
                            done: false,
                            task_id: None,
                        });
                    }
                } else {
                    // separate mode (default)
                    let lrc_path = filepath.with_extension("lrc");
                    if let Err(e) = tokio::fs::write(&lrc_path, lyric).await {
                        let _ = task.tx.send(DownloadEvent {
                            log: format!("⚠️ 歌词保存失败: {}", e),
                            done: false,
                            task_id: None,
                        });
                    } else {
                        let _ = task.tx.send(DownloadEvent {
                            log: format!("📝 歌词已保存: {}", lrc_path.display()),
                            done: false,
                            task_id: None,
                        });
                    }
                }
            }
            Ok(None) | Ok(Some(_)) => {}
            Err(e) => {
                let _ = task.tx.send(DownloadEvent {
                    log: format!("⚠️ 歌词获取失败: {}", e),
                    done: false,
                    task_id: None,
                });
            }
        }
    }

    Ok(filepath.to_string_lossy().to_string())
}

// ─── 网盘批量下载 ───────────────────────────────────────────────

async fn run_netdisk_batch(
    task: BatchTask,
    items: Vec<crate::models::BatchDownloadItem>,
    media_path: String,
) {
    let total = items.len();

    let _ = task.tx.send(DownloadEvent {
        log: format!("📦 网盘批量下载开始，共 {} 个链接", total),
        done: false,
        task_id: None,
    });

    for (i, item) in items.iter().enumerate() {
        let url = item.url.clone().unwrap_or_default();
        if url.is_empty() {
            let result = BatchDownloadResultItem {
                id: item.id.clone(),
                title: item.title.clone(),
                artist: item.artist.clone(),
                success: false,
                error: Some("缺少分享链接".into()),
                file_path: None,
            };
            task.items
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(result);
            task.failed
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            continue;
        }

        let _ = task.tx.send(DownloadEvent {
            log: format!("🔍 [{}/{}] 解析分享链接: {}", i + 1, total, url),
            done: false,
            task_id: None,
        });

        match netdisk_download_one(&url, &media_path, &task, i, total).await {
            Ok(paths) => {
                let file_names: Vec<String> = paths
                    .iter()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| p.clone())
                    })
                    .collect();
                let result = BatchDownloadResultItem {
                    id: item.id.clone(),
                    title: Some(file_names.join(", ")),
                    artist: item.artist.clone(),
                    success: true,
                    error: None,
                    file_path: Some(paths.join("\n")),
                };
                task.items
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(result);
                task.success
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            Err(e) => {
                let result = BatchDownloadResultItem {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    artist: item.artist.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    file_path: None,
                };
                task.items
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(result);
                task.failed
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = task.tx.send(DownloadEvent {
                    log: format!("❌ [{}/{}] 失败: {}", i + 1, total, e),
                    done: false,
                    task_id: None,
                });
            }
        }
    }

    let success = task.success.load(std::sync::atomic::Ordering::SeqCst);
    let failed = task.failed.load(std::sync::atomic::Ordering::SeqCst);
    let _ = task.tx.send(DownloadEvent {
        log: format!("🎉 网盘批量下载完成! 成功: {}, 失败: {}", success, failed),
        done: true,
        task_id: None,
    });

    task.running
        .store(false, std::sync::atomic::Ordering::SeqCst);
}

async fn netdisk_download_one(
    url: &str,
    media_path: &str,
    task: &BatchTask,
    idx: usize,
    total: usize,
) -> anyhow::Result<Vec<String>> {
    let info = netdisk::get_share_info(url).await?;

    let _ = task.tx.send(DownloadEvent {
        log: format!("📂 [{}/{}] 获取文件列表...", idx + 1, total),
        done: false,
        task_id: None,
    });

    let files = netdisk::list_share_files(&info).await?;
    if files.is_empty() {
        anyhow::bail!("分享中没有文件");
    }

    // Filter to audio files or all files if no audio found
    let audio_exts = [
        ".mp3", ".flac", ".m4a", ".wav", ".aac", ".ogg", ".opus", ".wma",
    ];
    let audio_files: Vec<_> = files
        .iter()
        .filter(|f| {
            let name_lower = f.filename.to_lowercase();
            audio_exts.iter().any(|ext| name_lower.ends_with(ext))
        })
        .collect();

    let files_to_download = if audio_files.is_empty() {
        files.iter().filter(|f| !f.is_dir).collect::<Vec<_>>()
    } else {
        audio_files
    };

    if files_to_download.is_empty() {
        anyhow::bail!("没有可下载的文件");
    }

    let _ = task.tx.send(DownloadEvent {
        log: format!(
            "📋 [{}/{}] 发现 {} 个文件",
            idx + 1,
            total,
            files_to_download.len()
        ),
        done: false,
        task_id: None,
    });

    let output_dir = PathBuf::from(media_path).join("downloads");
    tokio::fs::create_dir_all(&output_dir).await.ok();
    let mut downloaded_paths = Vec::new();

    for (fi, file) in files_to_download.iter().enumerate() {
        let _ = task.tx.send(DownloadEvent {
            log: format!(
                "⬇️ [{}/{}][{}/{}] 获取链接: {}",
                idx + 1,
                total,
                fi + 1,
                files_to_download.len(),
                file.filename
            ),
            done: false,
            task_id: None,
        });

        let link = match netdisk::get_download_link(&info, file.fs_id).await {
            Ok(l) => l,
            Err(e) => {
                let _ = task.tx.send(DownloadEvent {
                    log: format!("⚠️ 获取链接失败 {}: {}", file.filename, e),
                    done: false,
                    task_id: None,
                });
                continue;
            }
        };

        let safe_name = sanitize_filename(&file.filename);
        let output_path = output_dir.join(&safe_name);

        let _ = task.tx.send(DownloadEvent {
            log: format!(
                "⬇️ [{}/{}][{}/{}] 下载: {} ({} bytes)",
                idx + 1,
                total,
                fi + 1,
                files_to_download.len(),
                safe_name,
                file.size
            ),
            done: false,
            task_id: None,
        });

        match netdisk::download_file(&link, &output_path).await {
            Ok(size) => {
                let path_str = output_path.to_string_lossy().to_string();
                downloaded_paths.push(path_str.clone());
                let _ = task.tx.send(DownloadEvent {
                    log: format!(
                        "✅ [{}/{}][{}/{}] 已保存: {} ({} bytes)",
                        idx + 1,
                        total,
                        fi + 1,
                        files_to_download.len(),
                        safe_name,
                        size
                    ),
                    done: false,
                    task_id: None,
                });
            }
            Err(e) => {
                let _ = task.tx.send(DownloadEvent {
                    log: format!(
                        "❌ [{}/{}][{}/{}] 下载失败 {}: {}",
                        idx + 1,
                        total,
                        fi + 1,
                        files_to_download.len(),
                        safe_name,
                        e
                    ),
                    done: false,
                    task_id: None,
                });
            }
        }
    }

    if downloaded_paths.is_empty() {
        anyhow::bail!("没有文件下载成功");
    }

    Ok(downloaded_paths)
}
