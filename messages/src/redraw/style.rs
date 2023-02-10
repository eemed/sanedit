use super::Color;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct CellStyle {
    pub text_style: Option<TextStyle>,
    pub bg: Option<Color>,
    pub fg: Option<Color>,
}

pub fn merge_cell_styles(styles: &[Option<CellStyle>]) -> Option<CellStyle> {
    styles.into_iter().cloned().fold(None, merge_2_cell_styles)
}

fn merge_2_cell_styles(one: Option<CellStyle>, two: Option<CellStyle>) -> Option<CellStyle> {
    if one.is_none() {
        return two;
    }
    if two.is_none() {
        return one;
    }

    let mut one = one.unwrap();
    let two = two.unwrap();

    if let Some(bg) = two.bg {
        one.bg = Some(bg);
    }

    if let Some(fg) = two.fg {
        one.fg = Some(fg);
    }

    if let Some(s) = two.text_style {
        one.text_style = Some(s);
    }

    Some(one)
}
