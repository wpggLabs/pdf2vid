//! Small helper for running a child process while forwarding its stderr
//! progress output live.
//!
//! Model runtimes (Hugging Face downloads, tqdm bars) report progress on
//! stderr using carriage returns (`\r`) rather than newlines, so a
//! line-oriented reader would only see updates once a bar finishes. We
//! read raw bytes and split on both `\r` and `\n` so each progress update
//! is forwarded to the callback as it arrives.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};

/// Type of the per-segment progress callback. `Send + Sync` so it can be
/// held across `.await` inside Tauri command futures (which must be `Send`).
pub type ProgressFn<'a> = &'a (dyn Fn(&str) + Send + Sync);

/// Spawn `cmd`, streaming stderr segments to `on_progress` as they arrive
/// while draining stdout on a background thread (so a full stdout pipe
/// can't deadlock the stderr reader). Returns the exit status, the full
/// stdout text, and the full collected stderr text.
pub fn run_with_progress(
    mut cmd: Command,
    on_progress: ProgressFn<'_>,
) -> std::io::Result<(ExitStatus, String, String)> {
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn()?;

    // Drain stdout concurrently to avoid a pipe-full deadlock.
    let stdout_handle = child.stdout.take().map(|mut so| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = so.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        })
    });

    let mut collected = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        let mut buf = [0u8; 4096];
        let mut segment = String::new();
        loop {
            let n = stderr.read(&mut buf)?;
            if n == 0 {
                break;
            }
            let chunk = String::from_utf8_lossy(&buf[..n]);
            for ch in chunk.chars() {
                if ch == '\r' || ch == '\n' {
                    emit_segment(&mut segment, on_progress, &mut collected);
                } else {
                    segment.push(ch);
                }
            }
        }
        // Flush any trailing partial segment (no terminating separator).
        emit_segment(&mut segment, on_progress, &mut collected);
    }

    let stdout = stdout_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    let status = child.wait()?;
    Ok((status, stdout, collected))
}

fn emit_segment(segment: &mut String, on_progress: ProgressFn<'_>, collected: &mut String) {
    let trimmed = segment.trim();
    if !trimmed.is_empty() {
        on_progress(trimmed);
        collected.push_str(trimmed);
        collected.push('\n');
    }
    segment.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn forwards_carriage_return_progress_segments() {
        // A fake process that emits `\r`-delimited progress like tqdm does,
        // then a final newline-terminated line.
        let Some(python) = crate::edgetts::detect_python_with_edge_tts().or_else(find_python)
        else {
            eprintln!("skipping: no python available");
            return;
        };
        let mut cmd = Command::new(python);
        cmd.arg("-c").arg(
            "import sys\nfor i in range(0,101,25):\n sys.stderr.write('\\rProgress %d%%'%i)\n sys.stderr.flush()\nsys.stderr.write('\\ndone\\n')",
        );
        cmd.stdout(Stdio::null());

        let seen: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let (status, _stdout, collected) =
            run_with_progress(cmd, &|line| seen.lock().unwrap().push(line.to_string())).unwrap();

        assert!(status.success());
        let seen = seen.into_inner().unwrap();
        // Each \r-delimited progress update arrives as its own segment.
        assert!(
            seen.iter().any(|s| s.contains("Progress 0%")),
            "segments: {seen:?}"
        );
        assert!(seen.iter().any(|s| s.contains("Progress 100%")));
        assert!(seen.iter().any(|s| s == "done"));
        assert!(collected.contains("done"));
    }

    fn find_python() -> Option<std::path::PathBuf> {
        for name in ["python", "python3", "py"] {
            if Command::new(name)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Some(std::path::PathBuf::from(name));
            }
        }
        None
    }
}
