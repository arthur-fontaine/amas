use floem::{
    kurbo::{Line, Rect, Stroke},
    prelude::{palette::css, *},
    text::{Attrs, AttrsList, FamilyOwned, TextLayout},
};

use crate::file::File;

impl super::workspace_layout::WorkspaceLayout {
    pub fn draw(
        &self,
        cx: &mut floem::context::PaintCx<'_>,
        _size: floem::kurbo::Size,
    ) -> () {
        let zoom = self.view_state.zoom.get();
        let translation_x = self.view_state.translation_x.get();
        let translation_y = self.view_state.translation_y.get();

        let positions = self.calculate_positions();

        // Draw edges
        for pos in positions.iter() {
            let pos_u = &pos.1;
            for pos_v in &pos.2 {
                let x1 = pos_u.x * zoom + translation_x;
                let y1 = pos_u.y * zoom + translation_y;
                let x2 = pos_v.x * zoom + translation_x;
                let y2 = pos_v.y * zoom + translation_y;

                cx.stroke(
                    &Line::new((x1, y1), (x2, y2)),
                    css::WHITE,
                    &Stroke::new(4.0),
                );
            }
        }

        let mut files: Vec<(File, (f64, f64, f64, f64))> = vec![];
        // Draw nodes
        for pos in positions.iter() {
            let file = pos.0;
            let x = pos.1.x * zoom + translation_x;
            let y = pos.1.y * zoom + translation_y;
            let size = 40.0 * zoom;

            let rect = Rect::from_origin_size(
                (x - size / 2.0, y - size / 2.0),
                (size as f64, size as f64),
            );
            cx.fill(&rect, css::BLUE, 0.0);

            let mut text_layout = TextLayout::new();
            text_layout.set_text(
                &file.name.split('/').last().unwrap_or(&file.name),
                AttrsList::new(Attrs::new().family(&[FamilyOwned::SansSerif])),
            );
            cx.draw_text(&text_layout, (x, y));

            files.push((file.clone(), (x - size / 2.0, y - size / 2.0, x + size / 2.0, y + size / 2.0)));
        }
        self.canva_state.set_files(files);
    }
}
