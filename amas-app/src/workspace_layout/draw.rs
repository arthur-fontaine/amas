use floem::{
    kurbo::{Line, Rect, Stroke},
    prelude::{palette::css, *},
    text::{Attrs, AttrsList, FamilyOwned, TextLayout},
};

impl super::workspace_layout::WorkspaceLayout {
    pub fn draw(
        &self,
        cx: &mut floem::context::PaintCx<'_>,
        _size: floem::kurbo::Size,
    ) -> () {
        let positions = self.calculate_positions();

        // Draw edges
        for pos in positions.iter() {
            let pos_u = &pos.1;
            for pos_v in &pos.2 {
                cx.stroke(
                    &Line::new((pos_u.x, pos_u.y), (pos_v.x, pos_v.y)),
                    css::WHITE,
                    &Stroke::new(4.0),
                );
            }
        }

        // Draw nodes
        for pos in positions.iter() {
            let file = pos.0;
            let rect = Rect::from_origin_size(
                (pos.1.x - 20.0, pos.1.y - 20.0),
                (40.0, 40.0),
            );
            cx.fill(&rect, css::BLUE, 0.0);
            // cx.fill_text(&file.name, (pos.x - 20.0, pos.y - 20.0), css::WHITE, 12.0);

            let mut text_layout = TextLayout::new();
            text_layout.set_text(
                &file.name.split('/').last().unwrap_or(&file.name),
                AttrsList::new(Attrs::new().family(&[FamilyOwned::SansSerif])),
            );
            cx.draw_text(&text_layout, (pos.1.x, pos.1.y));
        }
    }
}
