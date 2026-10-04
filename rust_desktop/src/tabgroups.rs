//! Tab groups (a name and a colour shared by several tabs) and the "search tabs" popup.

use crate::app::App;
use crate::overlays;

/// `(key, emoji for menus, colour)`.
pub const GROUP_COLORS: [(&str, &str, [u8; 3]); 6] = [
    ("Red", "\u{1F534}", [220, 38, 38]),
    ("Orange", "\u{1F7E0}", [234, 88, 12]),
    ("Green", "\u{1F7E2}", [22, 163, 74]),
    ("Blue", "\u{1F535}", [37, 99, 235]),
    ("Purple", "\u{1F7E3}", [147, 51, 234]),
    ("Grey", "\u{26AB}", [100, 116, 139]),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabGroup {
    pub id: u32,
    pub name: String,
    pub color: usize,
}

/// A group name from user input: trimmed, no control characters, at most 24 characters.
pub fn clean_name(raw: &str, color: usize) -> String {
    let n: String = raw.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(24).collect();
    if n.is_empty() {
        GROUP_COLORS[color.min(GROUP_COLORS.len() - 1)].0.to_string()
    } else {
        n
    }
}

/// How a tab's group is written to the saved session: `"<colour index>|<name>"`, empty for no group.
pub fn group_key(group: &TabGroup) -> String {
    format!("{}|{}", group.color, group.name)
}

pub fn parse_group_key(key: &str) -> Option<(usize, String)> {
    let (c, name) = key.split_once('|')?;
    let color: usize = c.parse().ok().filter(|c| *c < GROUP_COLORS.len())?;
    Some((color, clean_name(name, color)))
}

impl App {
    pub fn group_color_of(&self, tab_idx: usize) -> Option<[u8; 3]> {
        let gid = self.tabs.get(tab_idx)?.group?;
        self.groups.iter().find(|g| g.id == gid).map(|g| GROUP_COLORS[g.color].2)
    }

    pub fn group_name_of(&self, tab_idx: usize) -> Option<String> {
        let gid = self.tabs.get(tab_idx)?.group?;
        self.groups.iter().find(|g| g.id == gid).map(|g| g.name.clone())
    }

    pub fn tab_group_key(&self, tab_idx: usize) -> String {
        let Some(gid) = self.tabs.get(tab_idx).and_then(|t| t.group) else { return String::new() };
        self.groups.iter().find(|g| g.id == gid).map(group_key).unwrap_or_default()
    }

    /// The group with this colour and name, created when it does not exist yet.
    pub fn group_for(&mut self, color: usize, name: &str) -> u32 {
        let name = clean_name(name, color);
        if let Some(g) = self.groups.iter().find(|g| g.color == color && g.name == name) {
            return g.id;
        }
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.push(TabGroup { id, name, color });
        id
    }

    pub fn restore_group(&mut self, key: &str) -> Option<u32> {
        let (color, name) = parse_group_key(key)?;
        Some(self.group_for(color, &name))
    }

    /// Forget groups that no tab uses any more.
    fn prune_groups(&mut self) {
        let used: Vec<u32> = self.tabs.iter().filter_map(|t| t.group).collect();
        self.groups.retain(|g| used.contains(&g.id));
    }

    /// Put the tab next to the other tabs of its group.
    fn gather_with_group(&mut self, tab_idx: usize) {
        let Some(gid) = self.tabs[tab_idx].group else { return };
        if self.tabs[tab_idx].pinned {
            return;
        }
        let last = self.tabs.iter().enumerate().filter(|(i, t)| *i != tab_idx && t.group == Some(gid) && !t.pinned).map(|(i, _)| i).max();
        if let Some(last) = last {
            let to = if tab_idx < last { last } else { last + 1 };
            let to = to.min(self.tabs.len() - 1);
            self.active = crate::tabs::move_tab(&mut self.tabs, self.active, tab_idx, to);
        }
    }

    pub fn tab_group_new(&mut self, tab_id: u64, color: usize, name: &str) {
        let Some(idx) = self.idx_of_id(tab_id) else { return };
        let gid = self.group_for(color.min(GROUP_COLORS.len() - 1), name);
        self.tabs[idx].group = Some(gid);
        self.after_group_change(tab_id);
    }

    pub fn tab_group_add(&mut self, tab_id: u64, gid: u32) {
        let Some(idx) = self.idx_of_id(tab_id) else { return };
        if self.groups.iter().any(|g| g.id == gid) {
            self.tabs[idx].group = Some(gid);
            self.after_group_change(tab_id);
        }
    }

    pub fn tab_ungroup(&mut self, tab_id: u64) {
        if let Some(idx) = self.idx_of_id(tab_id) {
            self.tabs[idx].group = None;
            self.after_group_change(tab_id);
        }
    }

    fn after_group_change(&mut self, tab_id: u64) {
        if let Some(idx) = self.idx_of_id(tab_id) {
            self.gather_with_group(idx);
        }
        self.prune_groups();
        self.redraw = true;
    }

    /// The popup that asks for a group's name and colour.
    pub fn open_group_prompt(&mut self, tab_id: u64) {
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            let _ = wv.focus();
            let _ = wv.evaluate_script(&overlays::group_prompt(self.core.theme, &shared.token, tab_id));
        }
    }

    // ------------------------------------------------------------------ search tabs

    pub fn open_tab_search(&mut self) {
        let rows: Vec<(u64, String, String, String, [u8; 3], bool)> = (0..self.tabs.len())
            .map(|i| {
                let t = &self.tabs[i];
                (t.id, t.title.clone(), t.url.clone(), self.group_name_of(i).unwrap_or_default(), self.group_color_of(i).unwrap_or([0, 0, 0]), i == self.active)
            })
            .collect();
        let shared = self.shared();
        if let Some(wv) = &self.webview {
            let _ = wv.focus();
            let _ = wv.evaluate_script(&overlays::tab_search_popup(self.core.theme, &shared.token, &rows));
        }
    }

    pub fn tab_search_go(&mut self, tab_id: u64) {
        if let Some(idx) = self.idx_of_id(tab_id) {
            self.activate(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cleaned_and_default_to_the_colour() {
        assert_eq!(clean_name("  Work \u{7}stuff ", 3), "Work stuff");
        assert_eq!(clean_name("   ", 3), "Blue");
        assert_eq!(clean_name("x".repeat(80).as_str(), 0).chars().count(), 24);
        assert_eq!(clean_name("", 99), "Grey", "an out-of-range colour is clamped");
    }

    #[test]
    fn group_keys_round_trip_and_reject_garbage() {
        let g = TabGroup { id: 1, name: "News | Assam".into(), color: 2 };
        assert_eq!(parse_group_key(&group_key(&g)), Some((2, "News | Assam".to_string())));
        assert_eq!(parse_group_key(""), None);
        assert_eq!(parse_group_key("9|x"), None);
        assert_eq!(parse_group_key("a|x"), None);
    }
}
