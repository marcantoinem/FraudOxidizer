#[derive(Clone, Copy)]
pub(super) enum ActionIcon {
    Previous,
    Next,
    Approve,
    Fraud,
}

pub(super) fn icon_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: ActionIcon,
    enabled: bool,
) -> egui::Response {
    let icon_size = 11.0;
    let icon_gap = 6.0;
    let text_color = if enabled {
        ui.visuals().widgets.inactive.fg_stroke.color
    } else {
        ui.visuals().widgets.noninteractive.fg_stroke.color
    };
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font_id.clone(), text_color);

    let padding = ui.spacing().button_padding;
    let desired_size = egui::vec2(
        padding.x * 2.0 + icon_size + icon_gap + galley.size().x,
        (padding.y * 2.0 + galley.size().y)
            .max(icon_size + padding.y * 2.0)
            .max(ui.spacing().interact_size.y),
    );

    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    let visuals = if enabled {
        ui.style().interact(&response)
    } else {
        &ui.visuals().widgets.noninteractive
    };

    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Middle,
    );

    let icon_center = egui::pos2(rect.left() + padding.x + icon_size * 0.5, rect.center().y);
    paint_action_icon(
        ui.painter(),
        icon_center,
        icon,
        icon_size,
        visuals.fg_stroke.color,
    );

    let text_pos = egui::pos2(
        rect.left() + padding.x + icon_size + icon_gap,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter()
        .galley(text_pos, galley, visuals.fg_stroke.color);

    response
}

fn paint_action_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    icon: ActionIcon,
    size: f32,
    color: egui::Color32,
) {
    let stroke = egui::Stroke::new(1.6, color);
    let half = size * 0.5;
    let q = size * 0.25;

    match icon {
        ActionIcon::Previous => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(center.x + q, center.y - half + 1.0),
                    egui::pos2(center.x - q, center.y),
                    egui::pos2(center.x + q, center.y + half - 1.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        ActionIcon::Next => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(center.x - q, center.y - half + 1.0),
                    egui::pos2(center.x + q, center.y),
                    egui::pos2(center.x - q, center.y + half - 1.0),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
        ActionIcon::Approve => {
            painter.line_segment(
                [
                    egui::pos2(center.x - half + 1.5, center.y + 0.5),
                    egui::pos2(center.x - q * 0.4, center.y + half - 1.5),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x - q * 0.4, center.y + half - 1.5),
                    egui::pos2(center.x + half - 1.0, center.y - half + 1.5),
                ],
                stroke,
            );
        }
        ActionIcon::Fraud => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(center.x, center.y - half + 1.0),
                    egui::pos2(center.x + half - 1.0, center.y + half - 1.0),
                    egui::pos2(center.x - half + 1.0, center.y + half - 1.0),
                ],
                color.gamma_multiply(0.2),
                stroke,
            ));
            painter.line_segment(
                [
                    egui::pos2(center.x, center.y - q * 0.9),
                    egui::pos2(center.x, center.y + q * 0.5),
                ],
                stroke,
            );
            painter.circle_filled(egui::pos2(center.x, center.y + half - 2.0), 1.0, color);
        }
    }
}

pub(super) fn paint_sort_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    descending: bool,
    color: egui::Color32,
) {
    let center = rect.center();
    let half_w = 3.5;
    let half_h = 2.5;

    let points = if descending {
        vec![
            egui::pos2(center.x - half_w, center.y - half_h),
            egui::pos2(center.x + half_w, center.y - half_h),
            egui::pos2(center.x, center.y + half_h),
        ]
    } else {
        vec![
            egui::pos2(center.x - half_w, center.y + half_h),
            egui::pos2(center.x + half_w, center.y + half_h),
            egui::pos2(center.x, center.y - half_h),
        ]
    };

    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}
