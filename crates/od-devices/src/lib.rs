//! USB devices: detection, registration and automatic incremental sync.

pub mod club_usb;
pub mod manager;
pub mod placement;
pub mod sync;
pub mod volumes;

pub use club_usb::{ClubUsbWriter, Proposal};
pub use manager::{Command, DeviceManager, DeviceState, DeviceStatus, Event, ManagerConfig};
pub use sync::{
    FormatWriter, NativeWriter, PlacedTrack, Progress, SyncContext, SyncError, SyncOptions,
    SyncReport,
};
pub use volumes::Volume;
