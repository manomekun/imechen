use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Checked,
    Indeterminate,
}

/// Selected rows keyed by path, so a row keeps its selection when the list
/// around it changes.
#[derive(Default)]
pub struct Selection {
    paths: HashSet<PathBuf>,
}

impl Selection {
    pub fn count(&self) -> usize {
        self.paths.len()
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.paths.contains(path)
    }

    pub fn toggle(&mut self, path: &Path) {
        if !self.paths.remove(path) {
            self.paths.insert(path.to_path_buf());
        }
    }

    /// State of the select-all checkbox over `all`.
    pub fn state_over<'a>(&self, all: impl IntoIterator<Item = &'a Path>) -> CheckState {
        let (mut total, mut selected) = (0, 0);
        for path in all {
            total += 1;
            if self.paths.contains(path) {
                selected += 1;
            }
        }
        match (total, selected) {
            (_, 0) => CheckState::Unchecked,
            (total, selected) if total == selected => CheckState::Checked,
            _ => CheckState::Indeterminate,
        }
    }

    /// Everything selected clears the selection; anything less selects it all.
    pub fn toggle_all<'a>(&mut self, all: impl IntoIterator<Item = &'a Path> + Clone) {
        if self.state_over(all.clone()) == CheckState::Checked {
            self.paths.clear();
        } else {
            self.paths = all.into_iter().map(Path::to_path_buf).collect();
        }
    }

    /// Drops selected paths that are no longer listed.
    pub fn retain_present<'a>(&mut self, present: impl IntoIterator<Item = &'a Path>) {
        let present: HashSet<&Path> = present.into_iter().collect();
        self.paths.retain(|path| present.contains(path.as_path()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    fn refs(paths: &[PathBuf]) -> impl Iterator<Item = &Path> + Clone {
        paths.iter().map(PathBuf::as_path)
    }

    #[test]
    fn select_all_checkbox_is_unchecked_when_nothing_is_selected() {
        let all = paths(&["a", "b"]);
        assert_eq!(
            Selection::default().state_over(refs(&all)),
            CheckState::Unchecked
        );
    }

    #[test]
    fn select_all_checkbox_is_unchecked_for_an_empty_list() {
        assert_eq!(
            Selection::default().state_over(std::iter::empty()),
            CheckState::Unchecked
        );
    }

    #[test]
    fn select_all_checkbox_is_indeterminate_on_partial_selection() {
        let all = paths(&["a", "b", "c"]);
        let mut selection = Selection::default();
        selection.toggle(Path::new("b"));
        assert_eq!(selection.state_over(refs(&all)), CheckState::Indeterminate);
    }

    #[test]
    fn select_all_checkbox_is_checked_when_every_row_is_selected() {
        let all = paths(&["a", "b"]);
        let mut selection = Selection::default();
        selection.toggle(Path::new("a"));
        selection.toggle(Path::new("b"));
        assert_eq!(selection.state_over(refs(&all)), CheckState::Checked);
    }

    #[test]
    fn toggling_a_selected_path_deselects_it() {
        let mut selection = Selection::default();
        selection.toggle(Path::new("a"));
        selection.toggle(Path::new("a"));
        assert_eq!(selection.count(), 0);
    }

    #[test]
    fn toggle_all_from_partial_selects_everything() {
        let all = paths(&["a", "b", "c"]);
        let mut selection = Selection::default();
        selection.toggle(Path::new("a"));
        selection.toggle_all(refs(&all));
        assert_eq!(selection.state_over(refs(&all)), CheckState::Checked);
        assert_eq!(selection.count(), 3);
    }

    #[test]
    fn toggle_all_from_everything_selected_clears_the_selection() {
        let all = paths(&["a", "b"]);
        let mut selection = Selection::default();
        selection.toggle_all(refs(&all));
        selection.toggle_all(refs(&all));
        assert_eq!(selection.count(), 0);
    }

    #[test]
    fn retain_present_prunes_paths_that_left_the_list() {
        let mut selection = Selection::default();
        selection.toggle(Path::new("a"));
        selection.toggle(Path::new("b"));
        selection.retain_present(refs(&paths(&["b", "c"])));
        assert!(!selection.contains(Path::new("a")));
        assert!(selection.contains(Path::new("b")));
        assert_eq!(selection.count(), 1);
    }

    #[test]
    fn duplicate_paths_share_one_selection() {
        let all = paths(&["a", "a"]);
        let mut selection = Selection::default();
        selection.toggle(Path::new("a"));
        assert_eq!(selection.state_over(refs(&all)), CheckState::Checked);
    }
}
