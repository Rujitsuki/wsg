#![allow(unused)]
use std::str::Chars;

pub struct BuildContext {
    pub size: Option<Size>,
    pub terminal_size: Size,
}

impl BuildContext {
    pub fn new(terminal_size: Size) -> Self {
        Self { size: None, terminal_size, }
    }

    pub fn size(&mut self, size: Size) {
        self.size = Some(size);
    }

}

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: Option<usize>,
    pub height: Option<usize>,
}

impl Size {
    pub fn new(w: usize, h: usize) -> Self {
        Self { width: Some(w), height: Some(h) }
    }

    pub fn only_width(w: usize) -> Self {
        Self { width: Some(w), height: None }
    }

    pub fn only_height(h: usize) -> Self {
        Self { width: None, height: Some(h) }
    }
}

pub struct UIBox<'a> {
    title: String,
    content: String,
    context: &'a BuildContext,
}

impl UIBox<'_> {
    pub fn new<T: Into<String>, C: Into<String>>(context: &BuildContext, title: T, content: C) -> UIBox<'_> {
        UIBox {
            context,
            title: title.into(),
            content: content.into(),
        }
    }

    pub fn render(self) {
        let computed_width = self.computed_width();

        self.render_header(computed_width);
        self.render_content(computed_width);
        self.render_footer(computed_width);
    }

    fn render_header(&self, computed_width: usize) {
        self.render_horizontal(computed_width, '┌', '┐', '─', self.title.as_ref());
    }

    fn render_content(&self, computed_width: usize) {
        let mut chars = &mut self.content.chars();
        let mut line_char_count = 0;

        fn fill_whitespace_and_close(line_char_count: usize, computed_width: usize) {
            let remaining_length = computed_width - line_char_count - 2;
            for _ in 0..remaining_length {
                print!(" ");
            }
            println!("│");
        }

        for ch in chars {
            if line_char_count == 0 {
                print!("│");
            }

            if ch == '\n' {
                fill_whitespace_and_close(line_char_count, computed_width);
                line_char_count = 0;
                continue;
            }

            if line_char_count >= computed_width -2 {
                print!("│");
                line_char_count = 0;
                continue;
            }

            print!("{}", ch);

            line_char_count += 1;
        }
        fill_whitespace_and_close(line_char_count, computed_width);
    }

    fn render_footer(&self, computed_width: usize) {
        self.render_horizontal(computed_width, '└', '┘', '─', None);
    }

    fn render_horizontal<'a, T: Into<Option<&'a str>>>(&self, computed_width: usize, start_char: char, end_char: char, between_char: char, text: T) {
        let text = text.into().unwrap_or("");
        let mut chars = text.chars();

        for width in 0..computed_width {
            if width == 0 {
                print!("{}", start_char);
                continue;
            }

            if width == computed_width - 1 {
                println!("{}", end_char);
                continue;
            }

            if let Some(char) = chars.next() {
                print!("{}", char);
            } else {
                print!("{}", between_char);
            }
        }
    }

    fn computed_width(&self) -> usize {
        let terminal_width = self.context.terminal_size.width.unwrap();
        let width = match self.context.size {
            Some(size) => size.width.unwrap_or(terminal_width),
            None => terminal_width,
        };

        width
    }
}
