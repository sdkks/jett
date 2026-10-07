use crate::ui::theme::Theme;
use unicode_width::UnicodeWidthChar;

#[derive(Clone)]
pub struct ThemeChoice {
    pub name: String,
    pub theme: Theme,
}

#[derive(Clone)]
pub struct ThemeSelector {
    pub choices: Vec<ThemeChoice>,
    pub filter: String,
    pub matches: Vec<usize>,
    pub highlighted: usize,
    pub original_theme: Theme,
    pub original_name: String,
}

impl ThemeSelector {
    pub fn new(choices: Vec<ThemeChoice>, theme: Theme, name: String) -> Self {
        let highlighted = choices
            .iter()
            .position(|choice| choice.name == name)
            .unwrap_or(0);
        let matches = (0..choices.len()).collect();
        Self {
            choices,
            filter: String::new(),
            matches,
            highlighted,
            original_theme: theme,
            original_name: name,
        }
    }

    pub fn selected(&self) -> Option<&ThemeChoice> {
        self.matches
            .get(self.highlighted)
            .map(|index| &self.choices[*index])
    }

    pub fn move_highlight(&mut self, down: bool) {
        if down {
            self.highlighted = (self.highlighted + 1).min(self.matches.len().saturating_sub(1));
        } else {
            self.highlighted = self.highlighted.saturating_sub(1);
        }
    }

    pub fn push(&mut self, character: char) {
        if !character.is_control() {
            self.filter.push(character);
            self.refilter();
        }
    }

    pub fn backspace(&mut self) {
        self.filter.pop();
        self.refilter();
    }

    fn refilter(&mut self) {
        let previous = self.selected().map(|choice| choice.name.clone());
        let filter = self.filter.to_lowercase();
        self.matches = self
            .choices
            .iter()
            .enumerate()
            .filter(|(_, choice)| choice.name.to_lowercase().contains(&filter))
            .map(|(index, _)| index)
            .collect();
        self.highlighted = previous
            .and_then(|name| {
                self.matches
                    .iter()
                    .position(|index| self.choices[*index].name == name)
            })
            .unwrap_or(0);
    }

    /// Keep the editable end visible without slicing UTF-8 or overflowing cells.
    pub fn visible_filter(&self, columns: usize) -> &str {
        let mut used = 0;
        let mut start = self.filter.len();
        for (index, character) in self.filter.char_indices().rev() {
            let width = character.width().unwrap_or(0);
            if used + width > columns {
                break;
            }
            used += width;
            start = index;
        }
        &self.filter[start..]
    }
}
