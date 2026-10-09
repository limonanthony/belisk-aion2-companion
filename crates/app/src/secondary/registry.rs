use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowKind {
    History,
    Detail,
    Report,
    Pvp,
    Todos,
    Resources,
    Settings,
    Prompt,
    Reminder,
    WhatsNew,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// A window of this kind was already open; nothing was created.
    Existing,
    Created,
}

/// The open secondary windows: at most one per kind.
pub struct Registry<W> {
    open: HashMap<WindowKind, W>,
}

impl<W> Default for Registry<W> {
    fn default() -> Self {
        Self { open: HashMap::new() }
    }
}

impl<W> Registry<W> {
    pub fn get(&self, kind: WindowKind) -> Option<&W> {
        self.open.get(&kind)
    }

    pub fn insert(&mut self, kind: WindowKind, window: W) {
        self.open.insert(kind, window);
    }

    pub fn remove(&mut self, kind: WindowKind) -> Option<W> {
        self.open.remove(&kind)
    }

    pub fn all(&self) -> impl Iterator<Item = &W> {
        self.open.values()
    }

    pub fn drain(&mut self) -> Vec<W> {
        self.open.drain().map(|(_, window)| window).collect()
    }
}

/// Runs `create` only when no window of `kind` is open, and keeps what it
/// returns. The registry is not borrowed while `create` runs: the new
/// window's first frame already asks which windows are open.
pub fn open_once<W, E>(
    registry: &RefCell<Registry<W>>,
    kind: WindowKind,
    create: impl FnOnce() -> Result<W, E>,
) -> Result<Opened, E> {
    if registry.borrow().get(kind).is_some() {
        return Ok(Opened::Existing);
    }
    let window = create()?;
    registry.borrow_mut().insert(kind, window);
    Ok(Opened::Created)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(registry: &RefCell<Registry<u32>>, kind: WindowKind, window: u32) -> Result<Opened, ()> {
        open_once(registry, kind, || Ok(window))
    }

    #[test]
    fn a_second_open_of_the_same_kind_creates_nothing() {
        let registry = RefCell::new(Registry::default());
        let mut created = 0;
        let mut open = |registry: &RefCell<Registry<u32>>, kind| {
            open_once::<u32, ()>(registry, kind, || {
                created += 1;
                Ok(created)
            })
        };

        assert_eq!(open(&registry, WindowKind::Pvp), Ok(Opened::Created));
        assert_eq!(open(&registry, WindowKind::Pvp), Ok(Opened::Existing));
        assert_eq!(created, 1);
        assert_eq!(registry.borrow().get(WindowKind::Pvp), Some(&1));
    }

    #[test]
    fn each_kind_has_its_own_window() {
        let registry = RefCell::new(Registry::default());
        open(&registry, WindowKind::History, 1).unwrap();
        open(&registry, WindowKind::Detail, 2).unwrap();

        assert_eq!(registry.borrow().get(WindowKind::History), Some(&1));
        assert_eq!(registry.borrow().get(WindowKind::Detail), Some(&2));
        assert_eq!(registry.borrow().get(WindowKind::Pvp), None);
    }

    #[test]
    fn a_closed_window_can_be_opened_again() {
        let registry = RefCell::new(Registry::default());
        open(&registry, WindowKind::Detail, 1).unwrap();
        registry.borrow_mut().remove(WindowKind::Detail);

        assert_eq!(open(&registry, WindowKind::Detail, 2), Ok(Opened::Created));
        assert_eq!(registry.borrow().get(WindowKind::Detail), Some(&2));
    }

    #[test]
    fn a_failed_creation_leaves_no_window_behind() {
        let registry: RefCell<Registry<u32>> = RefCell::default();

        assert_eq!(open_once(&registry, WindowKind::Pvp, || Err("no window")), Err("no window"));
        assert_eq!(registry.borrow().get(WindowKind::Pvp), None);
    }

    #[test]
    fn the_registry_can_be_read_while_a_window_is_being_created() {
        let registry = RefCell::new(Registry::default());

        let opened = open_once::<u32, ()>(&registry, WindowKind::Reminder, || {
            assert!(registry.borrow().get(WindowKind::Reminder).is_none());
            Ok(1)
        });

        assert_eq!(opened, Ok(Opened::Created));
    }

    #[test]
    fn draining_takes_every_window() {
        let registry = RefCell::new(Registry::default());
        open(&registry, WindowKind::History, 1).unwrap();
        open(&registry, WindowKind::Pvp, 2).unwrap();

        let mut all = registry.borrow_mut().drain();
        all.sort();

        assert_eq!(all, vec![1, 2]);
        assert_eq!(registry.borrow().get(WindowKind::History), None);
    }
}
