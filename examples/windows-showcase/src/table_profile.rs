//! Explicit native main-thread diagnostics. No capture/output without the env var.
use gpui_kit::perf_stats::Capture;
use std::{fs::File, io::Write};

pub(super) fn start() -> Option<(Capture, File)> {
    let path = std::env::var_os("YUE_TABLE_PROFILE_OUTPUT")?;
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        let file = File::options().write(true).create_new(true).open(path)?;
        Ok((Capture::start()?, file))
    })();
    match result {
        Ok(capture) => Some(capture),
        Err(error) => {
            eprintln!("Table diagnostic capture did not start: {error}");
            None
        }
    }
}

// Called after the native event loop returns: all scopes are closed, including
// input dispatch, frame cleanup and application teardown. Forced termination
// cannot produce a completed report; an empty output must never be accepted.
pub(super) fn finish(capture: Option<(Capture, File)>) {
    let Some((capture, mut file)) = capture else {
        return;
    };
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = capture.finish()?;
        writeln!(
            file,
            "TABLE_CAPTURE,elapsed_ns={},unattributed_ns={},dropped={}",
            snapshot.elapsed_ns(),
            snapshot.unattributed_ns(),
            snapshot.dropped()
        )?;
        for stage in snapshot.stages() {
            writeln!(
                file,
                "TABLE_STAGE,name={},calls={},inclusive_ns={},self_ns={},max_ns={}",
                stage.name(),
                stage.calls(),
                stage.inclusive_ns(),
                stage.self_ns(),
                stage.max_ns()
            )?;
        }
        file.flush()?;
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!("Table diagnostic capture did not finish: {error}");
    }
}
