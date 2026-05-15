pub mod capture;
pub mod config_env;
pub mod error;
pub mod portal;
pub mod smoothing;
pub mod sync_mode;

pub use capture::{
    CaptureBackend, CaptureProfile, CaptureStats, DetectedSampleCrop, GStreamerTestCapture,
    RgbAverage, SampleBenchFrame, SampleGridTiming, SamplePoint, SampleRegion,
};
pub use config_env::{
    config_home, home_dir, lumaway_main_env_path, migrate_lumaway_env_v1, read_env_file,
    upsert_env_file,
};
pub use error::{CoreError, Result};
pub use portal::{PortalScreenCast, PortalSelection, PortalStreamInfo};
pub use smoothing::{ColorSmoother, ColorSmoothingConfig};
pub use sync_mode::{
    config_v1_updates, SyncMode, CONFIG_VERSION_KEY, CURRENT_CONFIG_VERSION, LEGACY_PRESET_KEY,
    SYNC_MODE_KEY,
};
