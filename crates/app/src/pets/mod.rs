//! The pet collection the game reports and what shows it: `data` is the
//! per-server levels, souls and pins with the Global level rule, `store` reads
//! and writes `pets.json`, `service` is the `Pets` entity the bridge feeds and
//! the windows observe, `gain` is what it reports for each soul gain and
//! `settings` the switches that decide which gains the soul tracker card
//! shows. `list` is what the Pets window lists, `view` the window, `card` the
//! soul tracker card and `badge` the level badge both draw.

mod badge;
mod card;
mod data;
mod gain;
mod list;
mod service;
mod settings;
mod store;
mod view;

pub use card::{expected_size as card_size, watch_gains, CardRequest, PetCardView};
#[cfg(feature = "probes")]
pub use list::{ListChoice, Order, Show, Tab};
#[cfg(feature = "probes")]
pub use service::Pets;
pub use service::{global, install, record, ShowTestCard};
pub use settings::PetsSettings;
pub use view::{PetsView, EXPECTED_SIZE};
