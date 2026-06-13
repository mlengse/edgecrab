//! Queued prompt edit mode — Hermes `useQueue` + `cycleQueue` parity.

use crossterm::event::{self, KeyCode, KeyModifiers};

use super::App;

impl App {
    /// Exit queue edit: clear composer and history navigation state.
    pub(super) fn clear_queue_edit(&mut self) {
        self.queue_edit_idx = None;
        self.textarea_clear();
        self.history_pos = self.input_history.len();
        self.history_stash.clear();
    }

    /// Cycle the highlighted queue row (`dir`: `1` = up/next, `-1` = down/prev).
    pub(super) fn cycle_queue_edit(&mut self, dir: i32) -> bool {
        let len = self.prompt_queue.len();
        if len == 0 {
            return false;
        }
        let idx = match self.queue_edit_idx {
            None => {
                if dir > 0 {
                    0
                } else {
                    len - 1
                }
            }
            Some(cur) => (cur as i32 + dir).rem_euclid(len as i32) as usize,
        };
        self.queue_edit_idx = Some(idx);
        self.history_pos = self.input_history.len();
        self.history_stash.clear();
        let text = self.prompt_queue[idx].clone();
        self.textarea_set_text(&text);
        true
    }

    /// Delete the item under edit and exit edit mode.
    pub(super) fn remove_queue_edit_item(&mut self) {
        if let Some(idx) = self.queue_edit_idx {
            if idx < self.prompt_queue.len() {
                self.prompt_queue.remove(idx);
            }
            self.clear_queue_edit();
        }
    }

    /// Persist composer text into the queue row under edit (Hermes `replaceQueue`).
    pub(super) fn commit_queue_edit(&mut self, text: &str) -> bool {
        let Some(idx) = self.queue_edit_idx else {
            return false;
        };
        if idx >= self.prompt_queue.len() {
            self.clear_queue_edit();
            return true;
        }
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.prompt_queue.remove(idx);
        } else {
            self.prompt_queue[idx] = trimmed.to_string();
        }
        self.clear_queue_edit();
        true
    }

    /// Esc cancel / Ctrl+X delete while editing a queued prompt.
    pub(super) fn try_handle_queue_edit_key(&mut self, key: event::KeyEvent) -> bool {
        if self.queue_edit_idx.is_none() {
            return false;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.clear_queue_edit();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                self.remove_queue_edit_item();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    fn app_with_queue(items: &[&str]) -> App {
        let mut app = App::new();
        app.prompt_queue = items.iter().map(|s| s.to_string()).collect();
        app
    }

    #[tokio::test]
    async fn cycle_enters_edit_at_first_or_last() {
        let mut app = app_with_queue(&["a", "b", "c"]);
        assert!(app.cycle_queue_edit(1));
        assert_eq!(app.queue_edit_idx, Some(0));
        assert_eq!(app.textarea_text(), "a");

        app.queue_edit_idx = None;
        assert!(app.cycle_queue_edit(-1));
        assert_eq!(app.queue_edit_idx, Some(2));
        assert_eq!(app.textarea_text(), "c");
    }

    #[tokio::test]
    async fn commit_updates_row_and_clears_edit() {
        let mut app = app_with_queue(&["old"]);
        app.queue_edit_idx = Some(0);
        app.textarea_set_text("updated");
        assert!(app.commit_queue_edit("updated"));
        assert_eq!(app.prompt_queue, vec!["updated".to_string()]);
        assert_eq!(app.queue_edit_idx, None);
    }

    #[tokio::test]
    async fn remove_deletes_active_row() {
        let mut app = app_with_queue(&["one", "two"]);
        app.queue_edit_idx = Some(1);
        app.remove_queue_edit_item();
        assert_eq!(app.prompt_queue, vec!["one".to_string()]);
        assert_eq!(app.queue_edit_idx, None);
    }
}
