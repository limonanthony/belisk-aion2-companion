//! Images and icons compiled into the executable and served to GPUI.
//!
//! Icons are Lucide (ISC, `assets/icons/LICENSE`). Class icons are the game's
//! PNGs in `assets/game/class/`.

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Award,
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
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Self::Award => "icons/award.svg",
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
        }
    }
}

macro_rules! embedded {
    ($($path:literal => $file:literal),* $(,)?) => {
        &[$(($path, include_bytes!($file))),*]
    };
}

static ASSETS: &[(&str, &[u8])] = embedded! {
    "icons/award.svg" => "../assets/icons/award.svg",
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
    "class/assassin.png" => "../assets/game/class/assassin.png",
    "class/chanter.png" => "../assets/game/class/chanter.png",
    "class/cleric.png" => "../assets/game/class/cleric.png",
    "class/elementalist.png" => "../assets/game/class/elementalist.png",
    "class/fighter.png" => "../assets/game/class/fighter.png",
    "class/gladiator.png" => "../assets/game/class/gladiator.png",
    "class/ranger.png" => "../assets/game/class/ranger.png",
    "class/sorcerer.png" => "../assets/game/class/sorcerer.png",
    "class/templar.png" => "../assets/game/class/templar.png",
};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(ASSETS
            .iter()
            .find(|(known, _)| *known == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(ASSETS
            .iter()
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
        ];
        for icon in icons {
            assert!(Assets.load(icon.path()).unwrap().is_some_and(|bytes| !bytes.is_empty()), "{icon:?}");
        }
    }
}
