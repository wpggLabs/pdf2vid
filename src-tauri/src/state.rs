use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub active_job: Mutex<Option<JobHandle>>,
    pub model_download: Mutex<Option<ModelDownloadHandle>>,
}

pub struct JobHandle {
    pub job_id: String,
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct ModelDownloadHandle {
    pub model_id: String,
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

    /// Register a new model download. Replaces any in-flight download.
    pub async fn start_model_download(
        &self,
        model_id: String,
    ) -> Arc<AtomicBool> {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let mut current = self.model_download.lock().await;
        // If a previous download is still active, signal it to cancel so
        // it cleans up before being replaced.
        if let Some(prev) = current.as_ref() {
            prev.cancel_flag.store(true, Ordering::SeqCst);
        }
        *current = Some(ModelDownloadHandle {
            model_id,
            cancel_flag: cancel_flag.clone(),
        });
        cancel_flag
    }

    pub async fn finish_model_download(&self, model_id: &str) {
        let mut current = self.model_download.lock().await;
        if let Some(handle) = current.as_ref() {
            if handle.model_id == model_id {
                *current = None;
            }
        }
    }

    pub async fn cancel_model_download(&self) -> Option<String> {
        let current = self.model_download.lock().await;
        current.as_ref().map(|h| {
            h.cancel_flag.store(true, Ordering::SeqCst);
            h.model_id.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_model_download_replaces_in_flight() {
        let state = AppState::default();
        let flag_a = state.start_model_download("marian-en-es".into()).await;
        let flag_b = state.start_model_download("marian-en-fr".into()).await;

        // Starting a new download flips the previous cancel flag.
        assert!(flag_a.load(Ordering::SeqCst));
        assert!(!flag_b.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancel_model_download_signals_active_flag() {
        let state = AppState::default();
        let flag = state.start_model_download("piper-en_US-amy".into()).await;
        let id = state.cancel_model_download().await;
        assert_eq!(id.as_deref(), Some("piper-en_US-amy"));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn finish_model_download_clears_active() {
        let state = AppState::default();
        state.start_model_download("a".into()).await;
        state.finish_model_download("a").await;
        assert!(state.cancel_model_download().await.is_none());
    }
}