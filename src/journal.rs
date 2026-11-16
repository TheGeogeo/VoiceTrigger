use std::collections::VecDeque;
use std::time::Duration;

const MAX_ENTRIES: usize = 100;
const MAX_MESSAGE_CHARS: usize = 512;

#[derive(Default)]
pub struct Journal {
    entries: VecDeque<String>,
}

impl Journal {
    pub fn append(&mut self, elapsed: Duration, message: &str) {
        let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
        let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
        self.entries.push_back(format!(
            "[{:02}:{:02}.{:03}] {message}",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60,
            elapsed.subsec_millis()
        ));
        if self.entries.len() > MAX_ENTRIES {
            self.entries.pop_front();
        }
    }

    pub fn render(&self) -> String {
        self.entries
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\r\n")
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_keeps_only_recent_events_and_can_be_cleared() {
        let mut journal = Journal::default();
        for index in 0..=MAX_ENTRIES {
            journal.append(Duration::ZERO, &format!("event {index}"));
        }
        assert_eq!(journal.entries.len(), MAX_ENTRIES);
        assert!(journal.entries.front().unwrap().ends_with("event 1"));
        assert!(journal.entries.back().unwrap().ends_with("event 100"));
        journal.clear();
        assert!(journal.render().is_empty());
    }

    #[test]
    fn journal_bounds_unicode_messages_without_splitting_characters() {
        let mut journal = Journal::default();
        journal.append(Duration::from_millis(1250), &"é".repeat(2000));
        let text = journal.render();
        assert!(text.starts_with("[00:01.250] "));
        assert_eq!(text.matches('é').count(), MAX_MESSAGE_CHARS);
    }
}
