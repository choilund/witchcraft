use anyhow::Result;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn spawn_indexing_thread(db_name: PathBuf, assets: PathBuf) {
    std::thread::spawn(move || {
        let (tx, rx) = channel();

        // 1. Setup Watcher
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to initialize file watcher: {}", e);
                return;
            }
        };

        // Watch Antigravity and Claude directories 
        let home = std::env::var("HOME").unwrap_or_default();
        let gemini_dir = PathBuf::from(&home).join(".gemini/antigravity/brain");
        let claude_dir = PathBuf::from(&home).join(".claude/projects");
        
        let _ = watcher.watch(&gemini_dir, RecursiveMode::Recursive);
        let _ = watcher.watch(&claude_dir, RecursiveMode::Recursive);

        // 2. Local State Initialization (Thread-isolated DB and Models)
        let device = witchcraft::make_device();
        let embedder = match witchcraft::Embedder::new(&device, &assets) {
            Ok(e) => e,
            Err(err) => {
                log::error!("Watcher failed to load embeddings ML models: {}", err);
                return;
            }
        };

        // 3. Event Loop with Debouncing
        loop {
            // Block until an event occurs
            match rx.recv() {
                Ok(_) => {
                    // Drain any subsequent events that occur in the next 3 seconds (Debounce window)
                    std::thread::sleep(Duration::from_secs(3));
                    while rx.try_recv().is_ok() {}

                    // Execute Background Ingestion & Embedding
                    match run_background_pipeline(&db_name, &embedder, &device) {
                        Ok(_) => log::info!("Background ingestion successful"),
                        Err(e) => log::error!("Background indexing error: {}", e),
                    }
                }
                Err(_) => break, // Channel closed
            }
        }
    });
}

fn run_background_pipeline(
    db_name: &PathBuf,
    embedder: &witchcraft::Embedder,
    device: &candle_core::Device,
) -> Result<()> {
    // Check if new sessions exist
    let have_changes = crate::ingest(db_name)?;
    if have_changes {
        let db_rw = witchcraft::DB::new(db_name.clone())?;
        crate::embed_and_index(&db_rw, embedder, device)?;
    }
    Ok(())
}
