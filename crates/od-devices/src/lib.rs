//! USB devices: detection, registration and automatic incremental sync.

pub mod check;
pub mod club_usb;
mod convert;
pub mod manager;
pub mod placement;
pub mod sync;
pub mod volumes;

pub use club_usb::{ClubUsbWriter, Proposal};
pub use manager::{Command, DeviceManager, DeviceState, DeviceStatus, Event, ManagerConfig};
pub use sync::{
    FormatWriter, NativeWriter, PlacedTrack, Progress, SyncContext, SyncError, SyncOptions,
    SyncReport, stems_on_stick, stick_formats,
};
pub use volumes::Volume;
