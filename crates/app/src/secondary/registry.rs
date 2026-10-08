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

    /// Runs `create` only when no window of `kind` is open, and keeps what it
    /// returns.
    pub fn open<E>(&mut self, kind: WindowKind, create: impl FnOnce() -> Result<W, E>) -> Result<Opened, E> {
        if self.open.contains_key(&kind) {
            return Ok(Opened::Existing);
        }
        self.open.insert(kind, create()?);
        Ok(Opened::Created)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_open_of_the_same_kind_creates_nothing() {
        let mut registry = Registry::default();
        let mut created = 0;
        let mut open = |registry: &mut Registry<u32>, kind| {
            registry.open::<()>(kind, || {
                created += 1;
                Ok(created)
            })
        };

        assert_eq!(open(&mut registry, WindowKind::Pvp), Ok(Opened::Created));
        assert_eq!(open(&mut registry, WindowKind::Pvp), Ok(Opened::Existing));
        assert_eq!(created, 1);
        assert_eq!(registry.get(WindowKind::Pvp), Some(&1));
    }

    #[test]
    fn each_kind_has_its_own_window() {
        let mut registry = Registry::default();
        registry.open::<()>(WindowKind::History, || Ok("history")).unwrap();
        registry.open::<()>(WindowKind::Detail, || Ok("detail")).unwrap();

        assert_eq!(registry.get(WindowKind::History), Some(&"history"));
        assert_eq!(registry.get(WindowKind::Detail), Some(&"detail"));
        assert_eq!(registry.get(WindowKind::Pvp), None);
    }

    #[test]
    fn a_closed_window_can_be_opened_again() {
        let mut registry = Registry::default();
        registry.open::<()>(WindowKind::Detail, || Ok(1)).unwrap();
        registry.remove(WindowKind::Detail);

        assert_eq!(registry.open::<()>(WindowKind::Detail, || Ok(2)), Ok(Opened::Created));
        assert_eq!(registry.get(WindowKind::Detail), Some(&2));
    }

    #[test]
    fn a_failed_creation_leaves_no_window_behind() {
        let mut registry: Registry<u32> = Registry::default();

        assert_eq!(registry.open(WindowKind::Pvp, || Err("no window")), Err("no window"));
        assert_eq!(registry.get(WindowKind::Pvp), None);
    }

    #[test]
    fn draining_takes_every_window() {
        let mut registry = Registry::default();
        registry.open::<()>(WindowKind::History, || Ok(1)).unwrap();
        registry.open::<()>(WindowKind::Pvp, || Ok(2)).unwrap();

        let mut all = registry.drain();
        all.sort();

        assert_eq!(all, vec![1, 2]);
        assert_eq!(registry.get(WindowKind::History), None);
    }
}
