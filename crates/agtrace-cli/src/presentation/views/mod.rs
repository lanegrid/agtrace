// Views contain complex formatting logic that works with ViewModels
// They bridge the gap between ViewModels and the final output

pub mod doctor;
pub mod event;
pub mod pack;
pub mod session;

pub use doctor::*;
pub use event::*;
pub use pack::*;
pub use session::*;
