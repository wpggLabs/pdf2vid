use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub active_job: Mutex<Option<JobHandle>>,
}

pub struct JobHandle {
    pub job_id: String,
    pub cancel_flag: Arc<AtomicBool>,
}

impl AppState {
    pub async fn start_job(&self, job_id: String) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut active = self.active_job.lock().await;
        *active = Some(JobHandle {
            job_id: job_id.clone(),
            cancel_flag: cancel_flag.clone(),
        });
        cancel_flag
    }

    pub async fn finish_job(&self, job_id: &str) {
        let mut active = self.active_job.lock().await;
        if let Some(handle) = active.as_ref() {
            if handle.job_id == job_id {
                *active = None;
            }
        }
    }

    pub async fn cancel_job(&self) -> Option<String> {
        let active = self.active_job.lock().await;
        active.as_ref().map(|h| {
            h.cancel_flag.store(true, Ordering::SeqCst);
            h.job_id.clone()
        })
    }
}

pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("models");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn cache_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("cache");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}