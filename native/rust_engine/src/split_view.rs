//! Split-Screen Dual Browsing & Vertical Tabs Studio for Axomai Browser.
//! Allows simultaneous multi-tab side-by-side productivity with synchronized scrolling and sidebar tabs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitLayoutMode {
    Single,
    VerticalDual,
    HorizontalDual,
    QuadGrid,
}

#[derive(Debug, Clone)]
pub struct SplitPane {
    pub pane_id: u32,
    pub tab_id: u32,
    pub width_ratio: f32,
    pub is_scroll_synced: bool,
}

pub struct SplitViewEngine {
    pub mode: SplitLayoutMode,
    pub vertical_tabs_expanded: bool,
    pub panes: Vec<SplitPane>,
    pub active_pane_idx: usize,
}

impl SplitViewEngine {
    pub fn new() -> Self {
        SplitViewEngine {
            mode: SplitLayoutMode::Single,
            vertical_tabs_expanded: false,
            panes: vec![SplitPane {
                pane_id: 1,
                tab_id: 1,
                width_ratio: 1.0,
                is_scroll_synced: false,
            }],
            active_pane_idx: 0,
        }
    }

    pub fn set_split_mode(&mut self, mode: SplitLayoutMode, second_tab_id: Option<u32>) {
        self.mode = mode;
        match mode {
            SplitLayoutMode::Single => {
                self.panes.truncate(1);
                if let Some(first) = self.panes.first_mut() {
                    first.width_ratio = 1.0;
                }
            }
            SplitLayoutMode::VerticalDual | SplitLayoutMode::HorizontalDual => {
                let tab2 = second_tab_id.unwrap_or(2);
                self.panes = vec![
                    SplitPane { pane_id: 1, tab_id: 1, width_ratio: 0.5, is_scroll_synced: false },
                    SplitPane { pane_id: 2, tab_id: tab2, width_ratio: 0.5, is_scroll_synced: false },
                ];
            }
            SplitLayoutMode::QuadGrid => {
                self.panes = vec![
                    SplitPane { pane_id: 1, tab_id: 1, width_ratio: 0.5, is_scroll_synced: false },
                    SplitPane { pane_id: 2, tab_id: 2, width_ratio: 0.5, is_scroll_synced: false },
                    SplitPane { pane_id: 3, tab_id: 3, width_ratio: 0.5, is_scroll_synced: false },
                    SplitPane { pane_id: 4, tab_id: 4, width_ratio: 0.5, is_scroll_synced: false },
                ];
            }
        }
    }

    pub fn toggle_vertical_tabs(&mut self) -> bool {
        self.vertical_tabs_expanded = !self.vertical_tabs_expanded;
        self.vertical_tabs_expanded
    }

    pub fn toggle_scroll_sync(&mut self, pane_id: u32) {
        for pane in &mut self.panes {
            if pane.pane_id == pane_id {
                pane.is_scroll_synced = !pane.is_scroll_synced;
            }
        }
    }
}
