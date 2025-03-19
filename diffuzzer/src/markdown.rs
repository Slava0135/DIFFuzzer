/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

pub struct Markdown {
    content: String,
}

impl Markdown {
    pub fn new(title: String) -> Self {
        let text = title.replace("\n", " ");
        let underline = "=".repeat(text.len());
        Self {
            content: format!("{}\n{}\n\n", text, underline),
        }
    }
    pub fn heading(&mut self, text: String) {
        let text = text.replace("\n", " ");
        let underline = "-".repeat(text.len());
        self.content
            .push_str(&format!("{}\n{}\n\n", text, underline));
    }
    pub fn paragraph(&mut self, text: String) {
        let text = text.trim().replace("\n", "\n\n");
        self.content.push_str(&format!("{}\n\n", text));
    }
}

impl Display for Markdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading() {
        let mut md = Markdown::new("some\ntitle".to_owned());
        md.heading("some\nheading".to_owned());
        let expected = r#"
some title
==========

some heading
------------

"#
        .trim_start();
        assert_eq!(expected.to_owned(), md.to_string());
    }

    #[test]
    fn test_paragraph() {
        let mut md = Markdown::new("foobar".to_owned());
        md.paragraph("\nfirst para\nsecond para\n".to_owned());
        let expected = r#"
foobar
======

first para

second para

"#
        .trim_start();
        assert_eq!(expected.to_owned(), md.to_string());
    }
}
