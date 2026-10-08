//! Images and icons compiled into the executable and served to GPUI.
//!
//! Icons are Lucide (ISC, `assets/icons/LICENSE`). Class icons are the game's
//! PNGs in `assets/game/class/`.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Award,
    Bell,
    Capsule,
    Check,
    Close,
    Copy,
    Hourglass,
    LockOpen,
    Map,
    More,
    Reset,
    Target,
    Signal,
    TrendingDown,
    TrendingUp,
    ArrowDown,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    Clock,
    External,
    Eye,
    Heart,
    History,
    Info,
    Plus,
    Swords,
    Trash,
    User,
    Activity,
    CircleCheck,
    FileText,
    FolderOpen,
    Gauge,
    LoaderCircle,
    Minus,
    Play,
    Radio,
    Record,
    Scale,
    Server,
    ShieldCheck,
    SlidersHorizontal,
    Stop,
    TriangleAlert,
    Trophy,
    ListChecks,
    Undo,
    Users,
    Languages,
    Crown,
    Skull,
    ChevronLeft,
    MousePointer,
    Gamepad,
    Pencil,
    Package,
    Crosshair,
    Grip,
    Key,
    Zap,
    BellOff,
    ChevronsLeft,
    ChevronsRight,
    Settings,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Self::Award => "icons/award.svg",
            Self::Bell => "icons/bell.svg",
            Self::Capsule => "icons/chevrons-down-up.svg",
            Self::Check => "icons/check.svg",
            Self::Close => "icons/x.svg",
            Self::Copy => "icons/copy.svg",
            Self::Hourglass => "icons/hourglass.svg",
            Self::LockOpen => "icons/lock-open.svg",
            Self::Map => "icons/map.svg",
            Self::More => "icons/ellipsis.svg",
            Self::Reset => "icons/rotate-ccw.svg",
            Self::Target => "icons/target.svg",
            Self::Signal => "icons/signal-high.svg",
            Self::TrendingDown => "icons/trending-down.svg",
            Self::TrendingUp => "icons/trending-up.svg",
            Self::ArrowDown => "icons/arrow-down.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::Clock => "icons/clock.svg",
            Self::External => "icons/external-link.svg",
            Self::Eye => "icons/eye.svg",
            Self::Heart => "icons/heart.svg",
            Self::History => "icons/history.svg",
            Self::Info => "icons/info.svg",
            Self::Plus => "icons/plus.svg",
            Self::Swords => "icons/swords.svg",
            Self::Trash => "icons/trash-2.svg",
            Self::User => "icons/user.svg",
            Self::Activity => "icons/activity.svg",
            Self::CircleCheck => "icons/circle-check.svg",
            Self::FileText => "icons/file-text.svg",
            Self::FolderOpen => "icons/folder-open.svg",
            Self::Gauge => "icons/gauge.svg",
            Self::LoaderCircle => "icons/loader-circle.svg",
            Self::Minus => "icons/minus.svg",
            Self::Play => "icons/play.svg",
            Self::Radio => "icons/radio.svg",
            Self::Record => "icons/circle-small.svg",
            Self::Scale => "icons/scale.svg",
            Self::Server => "icons/server.svg",
            Self::ShieldCheck => "icons/shield-check.svg",
            Self::SlidersHorizontal => "icons/sliders-horizontal.svg",
            Self::Stop => "icons/square.svg",
            Self::TriangleAlert => "icons/triangle-alert.svg",
            Self::Trophy => "icons/trophy.svg",
            Self::ListChecks => "icons/list-checks.svg",
            Self::Undo => "icons/undo-2.svg",
            Self::Users => "icons/users.svg",
            Self::Languages => "icons/languages.svg",
            Self::Crown => "icons/crown.svg",
            Self::Skull => "icons/skull.svg",
            Self::ChevronLeft => "icons/chevron-left.svg",
            Self::MousePointer => "icons/mouse-pointer.svg",
            Self::Gamepad => "icons/gamepad-2.svg",
            Self::Pencil => "icons/pencil.svg",
            Self::Package => "icons/package.svg",
            Self::Crosshair => "icons/crosshair.svg",
            Self::Grip => "icons/grip-vertical.svg",
            Self::Key => "icons/key-round.svg",
            Self::Zap => "icons/zap.svg",
            Self::BellOff => "icons/bell-off.svg",
            Self::ChevronsLeft => "icons/chevrons-left.svg",
            Self::ChevronsRight => "icons/chevrons-right.svg",
            Self::Settings => "icons/settings.svg",
        }
    }
}

/// The game's combat power mark, shown beside a player's combat power.
pub const COMBAT_POWER: &str = "game/combat-power.png";

macro_rules! embedded {
    ($($path:literal => $file:literal),* $(,)?) => {
        &[$(($path, include_bytes!($file))),*]
    };
}

static ASSETS: &[(&str, &[u8])] = embedded! {
    "icons/award.svg" => "../assets/icons/award.svg",
    "icons/bell.svg" => "../assets/icons/bell.svg",
    "icons/chevrons-down-up.svg" => "../assets/icons/chevrons-down-up.svg",
    "icons/check.svg" => "../assets/icons/check.svg",
    "icons/x.svg" => "../assets/icons/x.svg",
    "icons/copy.svg" => "../assets/icons/copy.svg",
    "icons/hourglass.svg" => "../assets/icons/hourglass.svg",
    "icons/lock-open.svg" => "../assets/icons/lock-open.svg",
    "icons/rotate-ccw.svg" => "../assets/icons/rotate-ccw.svg",
    "icons/map.svg" => "../assets/icons/map.svg",
    "icons/ellipsis.svg" => "../assets/icons/ellipsis.svg",
    "icons/target.svg" => "../assets/icons/target.svg",
    "icons/signal-high.svg" => "../assets/icons/signal-high.svg",
    "icons/trending-down.svg" => "../assets/icons/trending-down.svg",
    "icons/trending-up.svg" => "../assets/icons/trending-up.svg",
    "icons/arrow-down.svg" => "../assets/icons/arrow-down.svg",
    "icons/chevron-down.svg" => "../assets/icons/chevron-down.svg",
    "icons/chevron-right.svg" => "../assets/icons/chevron-right.svg",
    "icons/chevron-up.svg" => "../assets/icons/chevron-up.svg",
    "icons/clock.svg" => "../assets/icons/clock.svg",
    "icons/external-link.svg" => "../assets/icons/external-link.svg",
    "icons/eye.svg" => "../assets/icons/eye.svg",
    "icons/heart.svg" => "../assets/icons/heart.svg",
    "icons/history.svg" => "../assets/icons/history.svg",
    "icons/info.svg" => "../assets/icons/info.svg",
    "icons/plus.svg" => "../assets/icons/plus.svg",
    "icons/swords.svg" => "../assets/icons/swords.svg",
    "icons/trash-2.svg" => "../assets/icons/trash-2.svg",
    "icons/user.svg" => "../assets/icons/user.svg",
    "icons/activity.svg" => "../assets/icons/activity.svg",
    "icons/circle-check.svg" => "../assets/icons/circle-check.svg",
    "icons/file-text.svg" => "../assets/icons/file-text.svg",
    "icons/folder-open.svg" => "../assets/icons/folder-open.svg",
    "icons/gauge.svg" => "../assets/icons/gauge.svg",
    "icons/loader-circle.svg" => "../assets/icons/loader-circle.svg",
    "icons/minus.svg" => "../assets/icons/minus.svg",
    "icons/play.svg" => "../assets/icons/play.svg",
    "icons/radio.svg" => "../assets/icons/radio.svg",
    "icons/circle-small.svg" => "../assets/icons/circle-small.svg",
    "icons/scale.svg" => "../assets/icons/scale.svg",
    "icons/server.svg" => "../assets/icons/server.svg",
    "icons/shield-check.svg" => "../assets/icons/shield-check.svg",
    "icons/sliders-horizontal.svg" => "../assets/icons/sliders-horizontal.svg",
    "icons/square.svg" => "../assets/icons/square.svg",
    "icons/triangle-alert.svg" => "../assets/icons/triangle-alert.svg",
    "icons/trophy.svg" => "../assets/icons/trophy.svg",
    "icons/list-checks.svg" => "../assets/icons/list-checks.svg",
    "icons/undo-2.svg" => "../assets/icons/undo-2.svg",
    "icons/users.svg" => "../assets/icons/users.svg",
    "icons/languages.svg" => "../assets/icons/languages.svg",
    "icons/crown.svg" => "../assets/icons/crown.svg",
    "icons/skull.svg" => "../assets/icons/skull.svg",
    "icons/chevron-left.svg" => "../assets/icons/chevron-left.svg",
    "icons/mouse-pointer.svg" => "../assets/icons/mouse-pointer.svg",
    "icons/gamepad-2.svg" => "../assets/icons/gamepad-2.svg",
    "icons/pencil.svg" => "../assets/icons/pencil.svg",
    "icons/package.svg" => "../assets/icons/package.svg",
    "icons/crosshair.svg" => "../assets/icons/crosshair.svg",
    "icons/grip-vertical.svg" => "../assets/icons/grip-vertical.svg",
    "icons/key-round.svg" => "../assets/icons/key-round.svg",
    "icons/zap.svg" => "../assets/icons/zap.svg",
    "icons/bell-off.svg" => "../assets/icons/bell-off.svg",
    "icons/chevrons-left.svg" => "../assets/icons/chevrons-left.svg",
    "icons/chevrons-right.svg" => "../assets/icons/chevrons-right.svg",
    "icons/settings.svg" => "../assets/icons/settings.svg",
    "class/assassin.png" => "../assets/game/class/assassin.png",
    "class/chanter.png" => "../assets/game/class/chanter.png",
    "class/cleric.png" => "../assets/game/class/cleric.png",
    "class/elementalist.png" => "../assets/game/class/elementalist.png",
    "class/fighter.png" => "../assets/game/class/fighter.png",
    "class/gladiator.png" => "../assets/game/class/gladiator.png",
    "class/ranger.png" => "../assets/game/class/ranger.png",
    "class/sorcerer.png" => "../assets/game/class/sorcerer.png",
    "class/templar.png" => "../assets/game/class/templar.png",
    "game/combat-power.png" => "../assets/game/profile_power_icon_pc.png",
};

/// Every PNG in `assets/game/skills`, listed by `build.rs`.
static SKILL_ICONS: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/skill_icons.rs"));

fn find(path: &str) -> Option<&'static [u8]> {
    ASSETS
        .iter()
        .chain(SKILL_ICONS)
        .find(|(known, _)| *known == path)
        .map(|(_, bytes)| *bytes)
}

pub fn has(path: &str) -> bool {
    find(path).is_some()
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(find(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ASSETS
            .iter()
            .chain(SKILL_ICONS)
            .filter(|(known, _)| known.starts_with(path))
            .map(|(known, _)| SharedString::from(*known))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_embedded() {
        let icons = [
            Icon::Award,
            Icon::Bell,
            Icon::Capsule,
            Icon::Check,
            Icon::Close,
            Icon::Copy,
            Icon::Hourglass,
            Icon::LockOpen,
            Icon::Map,
            Icon::More,
            Icon::Reset,
            Icon::Target,
            Icon::Signal,
            Icon::TrendingDown,
            Icon::TrendingUp,
            Icon::ArrowDown,
            Icon::ChevronDown,
            Icon::ChevronRight,
            Icon::ChevronUp,
            Icon::Clock,
            Icon::External,
            Icon::Eye,
            Icon::Heart,
            Icon::History,
            Icon::Info,
            Icon::Plus,
            Icon::Swords,
            Icon::Trash,
            Icon::User,
            Icon::Activity,
            Icon::CircleCheck,
            Icon::FileText,
            Icon::FolderOpen,
            Icon::Gauge,
            Icon::LoaderCircle,
            Icon::Minus,
            Icon::Play,
            Icon::Radio,
            Icon::Record,
            Icon::Scale,
            Icon::Server,
            Icon::ShieldCheck,
            Icon::SlidersHorizontal,
            Icon::Stop,
            Icon::TriangleAlert,
            Icon::Trophy,
            Icon::ListChecks,
            Icon::Undo,
            Icon::Users,
            Icon::Languages,
            Icon::Crown,
            Icon::Skull,
            Icon::ChevronLeft,
            Icon::MousePointer,
            Icon::Gamepad,
            Icon::Pencil,
            Icon::Package,
            Icon::Crosshair,
            Icon::Grip,
            Icon::Key,
            Icon::Zap,
            Icon::BellOff,
            Icon::ChevronsLeft,
            Icon::ChevronsRight,
            Icon::Settings,
        ];
        for icon in icons {
            assert!(Assets.load(icon.path()).unwrap().is_some_and(|bytes| !bytes.is_empty()), "{icon:?}");
        }
    }
}
