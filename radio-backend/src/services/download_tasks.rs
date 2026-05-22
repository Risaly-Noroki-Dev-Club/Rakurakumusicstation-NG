//! In-memory batch download task registry and task snapshots.

use crate::models::{BatchDownloadResultItem, DownloadEvent};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::broadcast;

pub(crate) struct BatchTaskSnapshot {
    pub(crate) running: bool,
    pub(crate) source: String,
    pub(crate) total: usize,
    pub(crate) success: usize,
    pub(crate) failed: usize,
    pub(crate) items: Vec<BatchDownloadResultItem>,
}

#[derive(Clone)]
pub(crate) struct BatchTask {
    pub(crate) tx: broadcast::Sender<DownloadEvent>,
    pub(crate) running: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) source: String,
    pub(crate) total: usize,
    pub(crate) success: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) failed: Arc<std::sync::atomic::AtomicUsize>,
    pub(crate) items: Arc<Mutex<Vec<BatchDownloadResultItem>>>,
}

impl BatchTask {
    pub(crate) fn new(source: String, total: usize) -> Self {
        let (tx, _rx) = broadcast::channel::<DownloadEvent>(512);
        Self {
            tx,
            running: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            source,
            total,
            success: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            failed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            items: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

pub(crate) fn batch_tasks() -> &'static Mutex<HashMap<String, BatchTask>> {
    static TASKS: OnceLock<Mutex<HashMap<String, BatchTask>>> = OnceLock::new();
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn insert_task(task_id: String, task: BatchTask) {
    let mut tasks = batch_tasks().lock().unwrap_or_else(|e| e.into_inner());
    tasks.insert(task_id, task);
}

pub(crate) fn remove_task(task_id: &str) {
    let mut tasks = batch_tasks().lock().unwrap_or_else(|e| e.into_inner());
    tasks.remove(task_id);
}

pub(crate) fn subscribe_task(task_id: &str) -> Option<broadcast::Receiver<DownloadEvent>> {
    let tasks = batch_tasks().lock().unwrap_or_else(|e| e.into_inner());
    tasks.get(task_id).map(|task| task.tx.subscribe())
}

pub(crate) fn task_snapshot(task_id: &str) -> Option<BatchTaskSnapshot> {
    let tasks = batch_tasks().lock().unwrap_or_else(|e| e.into_inner());
    let task = tasks.get(task_id)?;
    let items = task.items.lock().unwrap_or_else(|e| e.into_inner()).clone();

    Some(BatchTaskSnapshot {
        running: task.running.load(std::sync::atomic::Ordering::SeqCst),
        source: task.source.clone(),
        total: task.total,
        success: task.success.load(std::sync::atomic::Ordering::SeqCst),
        failed: task.failed.load(std::sync::atomic::Ordering::SeqCst),
        items,
    })
}

pub(crate) fn generate_task_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..12)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub(crate) fn quality_to_ncm_level(quality: &str) -> &'static str {
    match quality {
        "standard" => "standard",
        "high" => "higher",
        "exhigh" => "exhigh",
        "lossless" => "lossless",
        _ => "exhigh",
    }
}

pub(crate) fn ext_from_type(file_type: &str, url: &str) -> &'static str {
    if file_type == "flac" {
        "flac"
    } else if file_type == "mp3" {
        "mp3"
    } else if url.contains(".flac") {
        "flac"
    } else {
        "mp3"
    }
}
