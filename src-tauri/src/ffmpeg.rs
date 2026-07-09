use std::path::PathBuf;

pub fn ffmpeg_path() -> Option<PathBuf> {
    // Try bundled sidecar first (would be alongside the executable in production)
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(dir) = &exe_dir {
        for name in ["ffmpeg.exe", "ffmpeg"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Fall back to system PATH lookup
    which("ffmpeg")
}

pub fn ffprobe_path() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(dir) = &exe_dir {
        for name in ["ffprobe.exe", "ffprobe"] {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    which("ffprobe")
}

fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for path in std::env::split_paths(&paths) {
        for ext in ["", ".exe", ".bat", ".cmd"] {
            let candidate = path.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn check_ffmpeg() -> bool {
    let Some(path) = ffmpeg_path() else {
        return false;
    };
    let mut cmd = std::process::Command::new(path);
    cmd.arg("-version");
    crate::subprocess::hide_window(&mut cmd);
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn check_ffprobe() -> bool {
    let Some(path) = ffprobe_path() else {
        return false;
    };
    let mut cmd = std::process::Command::new(path);
    cmd.arg("-version");
    crate::subprocess::hide_window(&mut cmd);
    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

pub fn ensure_ffmpeg_or_error() -> Result<PathBuf, String> {
    ffmpeg_path().ok_or_else(|| {
        "FFmpeg is not installed. Install FFmpeg and ensure it is on your PATH, \
         or place the bundled sidecar binary next to the application."
            .to_string()
    })
}

pub fn ensure_ffprobe_or_error() -> Result<PathBuf, String> {
    ffprobe_path().ok_or_else(|| "FFprobe is not installed.".to_string())
}

/// Generate a silent MP3 of `seconds` length at the given output path.
/// Used as a fallback when voice synthesis fails so the export can still
/// produce a video (with silent audio) instead of aborting entirely.
pub fn generate_silence(seconds: f64, output: &std::path::Path) -> Result<(), String> {
    let ffmpeg = ensure_ffmpeg_or_error()?;
    let mut cmd = std::process::Command::new(ffmpeg);
    cmd.args([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anullsrc=r=44100:cl=mono",
        "-t",
        &format!("{seconds:.2}"),
        "-c:a",
        "libmp3lame",
        "-q:a",
        "4",
    ]);
    cmd.arg(output);
    crate::subprocess::hide_window(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("Failed to generate silent audio: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "Silent audio generation failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("unknown error")
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Youtube,
    Tiktok,
}

impl Aspect {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Aspect::Youtube => (1920, 1080),
            Aspect::Tiktok => (1080, 1920),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_dimensions() {
        assert_eq!(Aspect::Youtube.dimensions(), (1920, 1080));
        assert_eq!(Aspect::Tiktok.dimensions(), (1080, 1920));
    }
}
