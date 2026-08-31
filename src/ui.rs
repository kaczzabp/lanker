use crate::app::{LankerApp, View};
use crate::transfer::{format_bytes, PendingFile, TransferStatus};
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Margin, Rect, RichText, Sense,
    Stroke, Vec2,
};

// ─── Palette ────────────────────────────────────────────────────────────────

pub const BG_DARK: Color32 = Color32::from_rgb(9, 12, 16);        // #090C10
pub const PANEL_BG: Color32 = Color32::from_rgb(14, 18, 24);      // #0E1218
pub const SURFACE: Color32 = Color32::from_rgb(21, 27, 35);       // #151B23
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(28, 35, 45); // #1C232D
pub const BORDER: Color32 = Color32::from_rgb(38, 47, 60);        // #262F3C
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(226, 233, 240); // #E2E9F0
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(148, 163, 184); // #94A3B8
pub const TEXT_DIM: Color32 = Color32::from_rgb(93, 108, 126);    // #5D6C7E
pub const ACCENT: Color32 = Color32::from_rgb(16, 185, 129);      // emerald #10B981
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(52, 211, 153); // #34D399
pub const ACCENT_DIM: Color32 = Color32::from_rgb(4, 120, 87);    // #047857
pub const SUCCESS: Color32 = Color32::from_rgb(34, 197, 94);
pub const ERROR: Color32 = Color32::from_rgb(248, 113, 113);
pub const WARNING: Color32 = Color32::from_rgb(251, 191, 36);
pub const PROGRESS_BG: Color32 = Color32::from_rgb(30, 39, 50);
pub const ONLINE_DOT: Color32 = Color32::from_rgb(52, 211, 153);

const RADIUS: u8 = 10;
const RADIUS_SM: u8 = 7;

// ─── Theme ──────────────────────────────────────────────────────────────────

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = PANEL_BG;
    visuals.extreme_bg_color = Color32::from_rgb(6, 8, 11);
    visuals.faint_bg_color = SURFACE;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.25);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    visuals.window_corner_radius = CornerRadius::same(RADIUS + 2);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);

    visuals.text_cursor.stroke = Stroke::new(1.5_f32, ACCENT);
    visuals.hyperlink_color = ACCENT_HOVER;
    visuals.faint_bg_color = SURFACE;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.animation_time = 0.15;
    ctx.set_global_style(style);
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

// A clickable, fully custom-painted navigation row.
struct NavRow {
    resp: egui::Response,
    rect: egui::Rect,
}

// ─── Entry ──────────────────────────────────────────────────────────────────

pub fn render_ui(app: &mut LankerApp, ctx: &egui::Context) {
    render_sidebar(app, ctx);
    render_bottom_bar(app, ctx);
    render_view(app, ctx);

    if app.show_settings {
        render_settings_window(app, ctx);
    }

    render_incoming_request(app, ctx);
    handle_dropped_files(app, ctx);
}

// ─── Sidebar ────────────────────────────────────────────────────────────────

fn render_sidebar(app: &mut LankerApp, ctx: &egui::Context) {
    egui::SidePanel::left("app_sidebar")
        .resizable(false)
        .exact_width(214.0)
        .frame(
            egui::Frame::NONE
                .fill(BG_DARK)
                .inner_margin(Margin::symmetric(12, 14))
                .stroke(Stroke::new(1.0_f32, BORDER)),
        )
        .show(ctx, |ui| {
            ui.set_min_height(ui.available_height());

            // ── Logo ──
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                let r = CornerRadius::same(9);
                let mut mesh = egui::Mesh::default();
                mesh.add_colored_rect(rect, Color32::from_rgb(4, 120, 87));
                ui.painter().add(mesh);
                ui.painter().rect_stroke(rect, r, Stroke::new(1.0_f32, ACCENT), egui::StrokeKind::Inside);
                ui.painter().text(
                    rect.center() + Vec2::new(0.0, 0.5),
                    Align2::CENTER_CENTER,
                    "⚡",
                    FontId::proportional(16.0),
                    Color32::WHITE,
                );

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Lanker")
                            .size(17.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("LAN File Transfer")
                            .size(10.0)
                            .color(TEXT_DIM),
                    );
                });
            });

            ui.add_space(18.0);

            // ── Nav ──
            let active = app.active_view;
            let transfers_count = {
                let st = app.transfer_state.lock().unwrap();
                st.transfers.len()
            };
            let devices_count = app.devices.lock().unwrap().len();

            let nav_files = nav_item(ui, "files", "📁", "Files", View::Files == active, None);
            let nav_devices = nav_item(ui, "devices", "🖥", "Devices", View::Devices == active, (devices_count > 0).then_some(devices_count));
            let nav_transfers = nav_item(ui, "transfers", "⬆", "Transfers", View::Transfers == active, (transfers_count > 0).then_some(transfers_count));
            let nav_log = nav_item(ui, "log", "📋", "Log", View::Log == active, None);

            for (nav, view) in [
                (&nav_files, View::Files),
                (&nav_devices, View::Devices),
                (&nav_transfers, View::Transfers),
                (&nav_log, View::Log),
            ] {
                if nav.resp.clicked() {
                    app.active_view = view;
                }
            }

            // ── Sliding active/hover indicator ──
            let spots = [&nav_files, &nav_devices, &nav_transfers, &nav_log];
            let active_spot = match active {
                View::Files => spots[0],
                View::Devices => spots[1],
                View::Transfers => spots[2],
                View::Log => spots[3],
            };
            let target_spot = spots.iter().find(|n| n.resp.hovered()).copied().unwrap_or(active_spot);
            let target_y = target_spot.rect.center().y;
            let ind_y = ctx.animate_value_with_time(Id::new("nav_indicator_y"), target_y, 0.22);

            let bar_h = 20.0;
            let bar_x = nav_files.rect.min.x + 1.5;
            let bar = Rect::from_min_max(
                egui::pos2(bar_x, ind_y - bar_h / 2.0),
                egui::pos2(bar_x + 3.0, ind_y + bar_h / 2.0),
            );
            ui.painter().rect_filled(bar, CornerRadius::same(2), ACCENT);

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(10.0);

            // ── Bottom: settings + IP card ──
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(8.0);

                let settings = nav_pill(ui, "⚙", "Settings", app.show_settings);
                if settings.clicked() {
                    app.show_settings = !app.show_settings;
                }
                ui.add_space(6.0);

                let frame = egui::Frame::NONE
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(RADIUS_SM))
                    .inner_margin(Margin::symmetric(10, 9));
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 7.0), Sense::hover());
                        let c = rect.center();
                        ui.painter().circle_filled(c, 3.5, ONLINE_DOT);
                        ui.painter().circle_stroke(c, 6.0, Stroke::new(1.0_f32, ONLINE_DOT.gamma_multiply(0.35)));
                        ui.label(
                            RichText::new("Online")
                                .size(11.0)
                                .color(TEXT_SECONDARY)
                                .strong(),
                        );
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(&app.local_ip).size(11.0).color(TEXT_DIM));
                });
            });
        });
}

fn nav_pill(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> egui::Response {
    nav_row(ui, "settings", icon, label, active, None).resp
}

fn nav_item(
    ui: &mut egui::Ui,
    id_key: &str,
    icon: &str,
    label: &str,
    active: bool,
    badge: Option<usize>,
) -> NavRow {
    nav_row(ui, id_key, icon, label, active, badge)
}

fn nav_row(
    ui: &mut egui::Ui,
    id_key: &str,
    icon: &str,
    label: &str,
    active: bool,
    badge: Option<usize>,
) -> NavRow {
    let item_h = 30.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, item_h), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);

    let ctx = ui.ctx();
    let hover_t = ctx.animate_bool_responsive(Id::new(("nav_hover", id_key)), resp.hovered());
    let active_t = ctx.animate_bool_responsive(Id::new(("nav_active", id_key)), active);

    let radius = CornerRadius::same(RADIUS_SM);

    // Fill: transparent -> hover, plus active tint
    let bg_hover = SURFACE_HOVER;
    let bg_active = ACCENT.gamma_multiply(0.15);
    let fill = if active {
        lerp_color(bg_active, lerp_color(bg_active, bg_hover, 0.45), hover_t)
    } else {
        lerp_color(Color32::TRANSPARENT, bg_hover, hover_t)
    };
    ui.painter().rect_filled(rect, radius, fill);

    // Active soft border
    if active_t > 0.01 {
        let stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.55 * active_t));
        ui.painter().rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);
    }

    // Colors
    let icon_color = if active {
        lerp_color(ACCENT_HOVER, Color32::WHITE, hover_t * 0.5)
    } else {
        lerp_color(TEXT_DIM, TEXT_PRIMARY, hover_t)
    };
    let label_color = if active {
        lerp_color(ACCENT_HOVER, Color32::WHITE, hover_t * 0.35)
    } else {
        lerp_color(TEXT_SECONDARY, TEXT_PRIMARY, hover_t)
    };

    let icon_pos = rect.left_center() + Vec2::new(13.0, 0.0);
    ui.painter().text(icon_pos, Align2::LEFT_CENTER, icon, FontId::proportional(14.0), icon_color);

    let text_pos = icon_pos + Vec2::new(24.0, 0.0);
    ui.painter().text(
        text_pos,
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        label_color,
    );

    if let Some(n) = badge {
        let text = if n > 99 { "99+".to_string() } else { n.to_string() };
        let badge_rect = Rect::from_center_size(
            egui::pos2(rect.right() - 20.0, rect.center().y),
            Vec2::new(18.0, 18.0),
        );
        ui.painter().rect_filled(
            badge_rect,
            CornerRadius::same(9),
            ACCENT.gamma_multiply(0.18 + hover_t * 0.1),
        );
        ui.painter().text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(10.0),
            ACCENT_HOVER,
        );
    }

    NavRow { resp, rect }
}

// ─── Bottom bar ─────────────────────────────────────────────────────────────

fn render_bottom_bar(app: &mut LankerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_bar")
        .frame(
            egui::Frame::NONE
                .fill(PANEL_BG)
                .inner_margin(Margin::symmetric(18, 12))
                .stroke(Stroke::new(1.0_f32, BORDER)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let state = app.transfer_state.lock().unwrap();
                let total: u64 = app.pending_files.iter().map(|f| f.size).sum();

                // Queued pill
                stat_pill(ui, "📦", &format!("{} file{} · {}", app.pending_files.len(), if app.pending_files.len() == 1 { "" } else { "s" }, format_bytes(total)), TEXT_DIM);

                let is_busy = state.is_sending || state.is_receiving;

                if is_busy {
                    ui.add_space(6.0);
                    let speed_mb = state.speed_bytes_per_sec / (1024.0 * 1024.0);
                    let speed_color = if speed_mb > 80.0 { SUCCESS } else if speed_mb > 40.0 { WARNING } else { TEXT_SECONDARY };
                    stat_pill(ui, "⚡", &format!("{:.1} MB/s", speed_mb), speed_color);
                    stat_pill(ui, "⏳", &format!("{:.0}%", transfer_pct(&state)), ACCENT_HOVER);
                }

                drop(state);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if is_busy {
                        let cancel_btn = ui.add_sized(
                            [104.0, 34.0],
                            egui::Button::new(
                                RichText::new("■  Cancel").size(13.0).strong().color(Color32::WHITE),
                            )
                            .fill(ERROR)
                            .corner_radius(CornerRadius::same(9)),
                        );
                        if cancel_btn.clicked() {
                            app.send_cancel
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                            app.receive_cancel
                                .store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    } else {
                        let can_send = !app.pending_files.is_empty() && app.selected_device.is_some();
                        let btn_fill = if can_send { ACCENT } else { SURFACE };
                        let btn_text = if can_send { Color32::WHITE } else { TEXT_DIM };

                        let send_btn = ui.add_enabled(
                            can_send,
                            egui::Button::new(
                                RichText::new("📤  Send").size(14.0).strong().color(btn_text),
                            )
                            .fill(btn_fill)
                            .stroke(if can_send { Stroke::new(1.0_f32, ACCENT_HOVER) } else { Stroke::new(1.0_f32, BORDER) })
                            .corner_radius(CornerRadius::same(9))
                            .min_size(Vec2::new(128.0, 34.0)),
                        );
                        if send_btn.clicked() && can_send {
                            app.start_send_with_ctx(ctx);
                        }
                    }
                });
            });
        });
}

fn stat_pill(ui: &mut egui::Ui, icon: &str, text: &str, color: Color32) {
    let frame = egui::Frame::NONE
        .fill(SURFACE)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(Margin::symmetric(10, 5));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(11.0).color(TEXT_DIM));
            ui.label(RichText::new(text).size(12.0).color(color));
        });
    });
}

fn transfer_pct(state: &crate::transfer::TransferState) -> f32 {
    if state.total_bytes > 0 {
        (state.transferred_bytes as f32 / state.total_bytes as f32) * 100.0
    } else {
        0.0
    }
}

// ─── Views ──────────────────────────────────────────────────────────────────

fn render_view(app: &mut LankerApp, ctx: &egui::Context) {
    match app.active_view {
        View::Files => render_files_view(app, ctx),
        View::Devices => render_devices_view(app, ctx),
        View::Transfers => render_transfers_view(app, ctx),
        View::Log => render_log_view(app, ctx),
    }
}

fn view_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(2.0);
    ui.label(
        RichText::new(title)
            .size(20.0)
            .strong()
            .color(TEXT_PRIMARY),
    );
    ui.label(RichText::new(subtitle).size(12.0).color(TEXT_DIM));
    ui.add_space(14.0);
}

// ── Files view ─────────────────────────────────────────────────────────────

fn render_files_view(app: &mut LankerApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            view_header(ui, "Files", "Add files or folders to your transfer queue");

            // Action row
            ui.horizontal(|ui| {
                let add_btn = ui.add(
                    egui::Button::new(
                        RichText::new("➕  Add Files").size(12.5).color(TEXT_PRIMARY),
                    )
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(RADIUS_SM)),
                );
                if add_btn.clicked() {
                    if let Some(files) = rfd::FileDialog::new().pick_files() {
                        for f in files {
                            if let Some(pf) = PendingFile::from_path(f) {
                                app.pending_files.push(pf);
                            }
                        }
                    }
                }

                let folder_btn = ui.add(
                    egui::Button::new(
                        RichText::new("📁  Add Folder").size(12.5).color(TEXT_PRIMARY),
                    )
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(CornerRadius::same(RADIUS_SM)),
                );
                if folder_btn.clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        if let Some(pf) = PendingFile::from_path(folder) {
                            app.pending_files.push(pf);
                        }
                    }
                }

                if !app.pending_files.is_empty() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let clear_btn = ui.add(
                            egui::Button::new(
                                RichText::new("Clear all").size(11.5).color(TEXT_DIM),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0_f32, BORDER)),
                        );
                        if clear_btn.clicked() {
                            app.pending_files.clear();
                        }
                    });
                }
            });

            ui.add_space(16.0);

            if app.pending_files.is_empty() {
                render_drop_zone(ui);
            } else {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    render_pending_files(app, ui);
                });
            }
        });
}

fn render_drop_zone(ui: &mut egui::Ui) {
    let available = ui.available_size();
    let zone_height = (available.y - 20.0).max(220.0);

    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(available.x, zone_height),
        Sense::hover(),
    );
    let r = CornerRadius::same(14);
    let time = ui.input(|i| i.time);
    let pulse = ((time * 2.0).sin() * 0.5 + 0.5) as f32;
    let hover_t = ui
        .ctx()
        .animate_bool_responsive(Id::new("drop_zone_hover"), resp.hovered());
    let border_alpha = 0.35 + pulse * 0.25 + hover_t * 0.4;
    let border_color = ACCENT.gamma_multiply(border_alpha.min(1.0));

    ui.painter().rect_filled(rect, r, ACCENT.gamma_multiply(0.04 + hover_t * 0.06));
    ui.painter().rect_stroke(rect, r, Stroke::new(1.6_f32, border_color), egui::StrokeKind::Inside);

    let center = rect.center();

    // Icon in a soft circle
    let icon_r = 34.0;
    ui.painter().circle_filled(center - Vec2::new(0.0, 42.0), icon_r, ACCENT.gamma_multiply(0.12));
    ui.painter().circle_stroke(center - Vec2::new(0.0, 42.0), icon_r, Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.5)));
    ui.painter().text(
        center - Vec2::new(0.0, 42.0),
        Align2::CENTER_CENTER,
        "⬇",
        FontId::proportional(26.0),
        ACCENT_HOVER,
    );

    ui.painter().text(
        center + Vec2::new(0.0, 26.0),
        Align2::CENTER_CENTER,
        "Drop files here",
        FontId::proportional(16.0),
        TEXT_PRIMARY,
    );
    ui.painter().text(
        center + Vec2::new(0.0, 48.0),
        Align2::CENTER_CENTER,
        "or use the buttons above · files & folders supported",
        FontId::proportional(12.0),
        TEXT_DIM,
    );
}

fn render_pending_files(app: &mut LankerApp, ui: &mut egui::Ui) {
    let mut to_remove = None;

    for (i, file) in app.pending_files.iter().enumerate() {
        let frame = egui::Frame::NONE
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .corner_radius(CornerRadius::same(RADIUS_SM))
            .inner_margin(Margin::symmetric(12, 9));
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Icon tile
                let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(7), ACCENT.gamma_multiply(0.10));
                let icon = if file.is_dir { "📁" } else { "📄" };
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    icon,
                    FontId::proportional(15.0),
                    ACCENT_HOVER,
                );

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&file.display_name)
                            .size(13.0)
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(if file.is_dir { "Folder · " } else { "File · " }.to_string() + &format_bytes(file.size))
                            .size(11.0)
                            .color(TEXT_DIM),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let remove_btn = ui.add(
                        egui::Button::new(RichText::new("✕").size(12.0).color(TEXT_DIM))
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(CornerRadius::same(5)),
                    );
                    if remove_btn.clicked() {
                        to_remove = Some(i);
                    }
                });
            });
        });
        ui.add_space(6.0);
    }

    if let Some(idx) = to_remove {
        app.pending_files.remove(idx);
    }
}

// ── Devices view ────────────────────────────────────────────────────────────

fn render_devices_view(app: &mut LankerApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            view_header(ui, "Devices", "Select a device to receive your files");

            let devices = app.devices.lock().unwrap().clone();

            if devices.is_empty() {
                let available = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(available.x, (available.y - 20.0).max(180.0)),
                    Sense::hover(),
                );
                let r = CornerRadius::same(14);
                ui.painter().rect_filled(rect, r, SURFACE);
                ui.painter().rect_stroke(rect, r, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);

                let center = rect.center();
                ui.painter().text(
                    center - Vec2::new(0.0, 30.0),
                    Align2::CENTER_CENTER,
                    "📡",
                    FontId::proportional(40.0),
                    TEXT_DIM,
                );
                let time = ui.input(|i| i.time);
                let dots = match (time * 2.0) as usize % 4 {
                    0 => "Scanning",
                    1 => "Scanning.",
                    2 => "Scanning..",
                    _ => "Scanning...",
                };
                ui.painter().text(
                    center + Vec2::new(0.0, 12.0),
                    Align2::CENTER_CENTER,
                    dots,
                    FontId::proportional(14.0),
                    TEXT_SECONDARY,
                );
                ui.painter().text(
                    center + Vec2::new(0.0, 34.0),
                    Align2::CENTER_CENTER,
                    "Waiting for devices on your network",
                    FontId::proportional(11.0),
                    TEXT_DIM,
                );
            } else {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (i, device) in devices.iter().enumerate() {
                            render_device_card(app, ui, i, device);
                        }
                    });
                });
            }
        });
}

fn render_device_card(
    app: &mut LankerApp,
    ui: &mut egui::Ui,
    idx: usize,
    device: &crate::network::DiscoveredDevice,
) {
    let is_selected = app.selected_device == Some(idx);
    let card_w = 230.0;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(card_w, 92.0),
        Sense::click(),
    );
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let r = CornerRadius::same(RADIUS_SM + 1);
    let ctx = ui.ctx();

    let hover_t = ctx.animate_bool_responsive(
        Id::new(("dev_hover", device.ip.as_str(), device.port)),
        resp.hovered(),
    );
    let lift = -hover_t * 2.5;
    let lifted = rect.translate(Vec2::new(0.0, lift));

    let fill = if is_selected {
        ACCENT.gamma_multiply(0.12)
    } else {
        lerp_color(SURFACE, SURFACE_HOVER, hover_t)
    };
    let stroke = if is_selected {
        Stroke::new(1.6_f32, ACCENT)
    } else {
        Stroke::new(1.0_f32, lerp_color(BORDER, ACCENT, hover_t * 0.55))
    };

    // Soft glow ring while hovering
    if hover_t > 0.01 && !is_selected {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            r,
            Stroke::new(1.0_f32, ACCENT.gamma_multiply(hover_t * 0.25)),
            egui::StrokeKind::Outside,
        );
    }

    ui.painter().rect_filled(lifted, r, fill);
    ui.painter().rect_stroke(lifted, r, stroke, egui::StrokeKind::Inside);

    let base = lifted.min;

    // Avatar
    let initials = device_name_initials(&device.name);
    let avatar_r = 20.0;
    let avatar_center = base + Vec2::new(14.0 + 20.0, 46.0);
    let color = hash_color(&device.name);
    ui.painter().circle_filled(avatar_center, avatar_r, color.gamma_multiply(0.25));
    ui.painter().circle_stroke(avatar_center, avatar_r, Stroke::new(1.0_f32, color));
    ui.painter().text(
        avatar_center,
        Align2::CENTER_CENTER,
        initials,
        FontId::proportional(13.0),
        color,
    );

    // Online dot
    let dot_center = avatar_center + Vec2::new(avatar_r - 4.0, avatar_r - 4.0);
    ui.painter().circle_filled(dot_center, 5.0, ONLINE_DOT);
    ui.painter().circle_stroke(dot_center, 5.0, Stroke::new(2.0_f32, fill));

    // Text (cleared from the avatar's right edge at 54.0)
    let text_x = base.x + 62.0;
    ui.painter().text(
        egui::pos2(text_x, base.y + 30.0),
        Align2::LEFT_CENTER,
        &device.name,
        FontId::proportional(13.0),
        TEXT_PRIMARY,
    );
    ui.painter().text(
        egui::pos2(text_x, base.y + 52.0),
        Align2::LEFT_CENTER,
        format!("{}:{}", device.ip, device.port),
        FontId::proportional(11.0),
        TEXT_DIM,
    );

    // Selected checkmark
    if is_selected {
        let check_pos = lifted.right_top() + Vec2::new(-16.0, 16.0);
        ui.painter().circle_filled(check_pos, 10.0, ACCENT);
        ui.painter().text(
            check_pos,
            Align2::CENTER_CENTER,
            "✓",
            FontId::proportional(12.0),
            Color32::WHITE,
        );
    }

    ui.add_space(10.0);

    if resp.clicked() {
        app.selected_device = Some(idx);
    }
}

fn device_name_initials(name: &str) -> String {
    let mut chars = name.chars().filter(|c| c.is_alphanumeric());
    let first = chars.next().unwrap_or('?');
    let second = chars.next().unwrap_or(' ');
    if second == ' ' {
        first.to_string()
    } else {
        format!("{first}{second}").to_uppercase()
    }
}

fn hash_color(name: &str) -> Color32 {
    let mut h: u32 = 0;
    for b in name.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u32);
    }
    let palette = [
        Color32::from_rgb(16, 185, 129),
        Color32::from_rgb(96, 165, 250),
        Color32::from_rgb(250, 204, 21),
        Color32::from_rgb(251, 113, 133),
        Color32::from_rgb(167, 139, 250),
        Color32::from_rgb(45, 212, 191),
        Color32::from_rgb(251, 146, 60),
    ];
    palette[(h as usize) % palette.len()]
}

// ── Transfers view ──────────────────────────────────────────────────────────

fn render_transfers_view(app: &mut LankerApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            view_header(ui, "Transfers", "Live progress of outgoing and incoming transfers");

            let state = app.transfer_state.lock().unwrap();

            if state.transfers.is_empty() && !state.is_sending && !state.is_receiving {
                let available = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(available.x, (available.y - 20.0).max(180.0)),
                    Sense::hover(),
                );
                let r = CornerRadius::same(14);
                ui.painter().rect_filled(rect, r, SURFACE);
                ui.painter().rect_stroke(rect, r, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
                let center = rect.center();
                ui.painter().text(
                    center - Vec2::new(0.0, 28.0),
                    Align2::CENTER_CENTER,
                    "📭",
                    FontId::proportional(38.0),
                    TEXT_DIM,
                );
                ui.painter().text(
                    center + Vec2::new(0.0, 10.0),
                    Align2::CENTER_CENTER,
                    "No transfers yet",
                    FontId::proportional(14.0),
                    TEXT_SECONDARY,
                );
                ui.painter().text(
                    center + Vec2::new(0.0, 32.0),
                    Align2::CENTER_CENTER,
                    "Send files from the Files view",
                    FontId::proportional(11.0),
                    TEXT_DIM,
                );
            } else {
                // Stats row
                let completed = state.transfers.iter().filter(|t| t.status == TransferStatus::Complete).count();
                let failed = state.transfers.iter().filter(|t| matches!(t.status, TransferStatus::Failed(_))).count();
                let total = state.transfers.len();

                ui.horizontal(|ui| {
                    stat_card(ui, "Total", &total.to_string(), TEXT_SECONDARY);
                    ui.add_space(8.0);
                    stat_card(ui, "Done", &completed.to_string(), SUCCESS);
                    ui.add_space(8.0);
                    if failed > 0 {
                        stat_card(ui, "Failed", &failed.to_string(), ERROR);
                        ui.add_space(8.0);
                    }
                    if state.is_sending || state.is_receiving {
                        let speed_mb = state.speed_bytes_per_sec / (1024.0 * 1024.0);
                        stat_card(ui, "Speed", &format!("{:.1} MB/s", speed_mb), ACCENT_HOVER);
                        ui.add_space(8.0);
                        stat_card(ui, "Overall", &format!("{:.0}%", transfer_pct(&state)), TEXT_SECONDARY);
                    }
                });
                ui.add_space(16.0);

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (i, transfer) in state.transfers.iter().enumerate() {
                        render_transfer_item(ui, i, transfer);
                        ui.add_space(6.0);
                    }
                });
            }
        });
}

fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    let frame = egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(RADIUS_SM))
        .inner_margin(Margin::symmetric(14, 8));
    frame.show(ui, |ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label.to_uppercase()).size(10.0).color(TEXT_DIM).strong());
            ui.label(RichText::new(value).size(17.0).strong().color(color));
        });
    });
}

fn render_transfer_item(ui: &mut egui::Ui, index: usize, transfer: &crate::transfer::TransferInfo) {
    let frame = egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(RADIUS_SM))
        .inner_margin(Margin::symmetric(12, 9));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());

        ui.horizontal(|ui| {
            let (icon, color) = match &transfer.status {
                TransferStatus::Pending => ("⏳", TEXT_DIM),
                TransferStatus::InProgress => ("⬆", ACCENT),
                TransferStatus::Complete => ("✓", SUCCESS),
                TransferStatus::Failed(_) => ("✕", ERROR),
                TransferStatus::Cancelled => ("⊘", WARNING),
            };

            let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(7), color.gamma_multiply(0.12));
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon, FontId::proportional(14.0), color);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&transfer.filename).size(12.5).color(TEXT_PRIMARY));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} / {}",
                                format_bytes(transfer.transferred),
                                format_bytes(transfer.total_size)
                            ))
                            .size(11.0)
                            .color(TEXT_DIM),
                        );
                    });
                });

                if transfer.total_size > 0 {
                    let target = transfer.transferred as f32 / transfer.total_size as f32;
                    // Smoothly chase the real progress so the bar slides, not jumps
                    let progress = ui.ctx().animate_value_with_time(
                        Id::new(("transfer_progress", index, &transfer.filename)),
                        target,
                        0.35,
                    );
                    let bar_height = 5.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_height), Sense::hover());
                    let r = CornerRadius::same(3);

                    ui.painter().rect_filled(rect, r, PROGRESS_BG);
                    ui.painter().rect_stroke(rect, r, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);

                    let fill_width = rect.width() * progress.clamp(0.0, 1.0);
                    if fill_width > 1.0 {
                        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_width, bar_height));
                        let fill_color = match &transfer.status {
                            TransferStatus::Complete => SUCCESS,
                            TransferStatus::Failed(_) => ERROR,
                            TransferStatus::Cancelled => WARNING,
                            _ => ACCENT,
                        };
                        ui.painter().rect_filled(fill_rect, r, fill_color);
                    }

                    // Glow tip for in-progress bars
                    if matches!(transfer.status, TransferStatus::InProgress) && fill_width > 2.0 {
                        let glow_alpha = (ui.input(|i| i.time) * 4.0).sin().abs() as f32;
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                egui::pos2(rect.min.x + fill_width - 2.0, rect.min.y),
                                egui::pos2(rect.min.x + fill_width + 2.0, rect.max.y),
                            ),
                            r,
                            ACCENT_HOVER.gamma_multiply(0.4 + glow_alpha * 0.6),
                        );
                    }
                }

                if let TransferStatus::Failed(msg) = &transfer.status {
                    ui.label(RichText::new(msg).size(10.5).color(ERROR));
                }
            });
        });
    });
}

// ── Log view ────────────────────────────────────────────────────────────────

fn render_log_view(app: &mut LankerApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            view_header(ui, "Log", "Activity history for this session");

            let state = app.transfer_state.lock().unwrap();

            let frame = egui::Frame::NONE
                .fill(SURFACE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(RADIUS_SM))
                .inner_margin(Margin::same(12));
            frame.show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        if state.log_messages.is_empty() {
                            ui.label(RichText::new("No log messages yet").size(12.0).color(TEXT_DIM));
                        } else {
                            for msg in state.log_messages.iter().rev() {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("»").size(12.0).color(ACCENT.gamma_multiply(0.7)));
                                    ui.label(RichText::new(msg).size(12.0).color(TEXT_SECONDARY));
                                });
                                ui.add_space(4.0);
                            }
                        }
                    });
            });
        });
}

// ─── Settings window ────────────────────────────────────────────────────────

/// A delicate settings row: tiny icon + caption above a soft, tinted input tile.
fn settings_field(ui: &mut egui::Ui, icon: &str, label: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(RichText::new(icon).size(12.0).color(ACCENT_HOVER.gamma_multiply(0.8)));
        ui.label(
            RichText::new(label.to_uppercase())
                .size(9.5)
                .color(TEXT_DIM)
                .strong(),
        );
    });
    ui.add_space(5.0);

    egui::Frame::NONE
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER.gamma_multiply(0.8)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });

    ui.add_space(14.0);
}

fn render_settings_window(app: &mut LankerApp, ctx: &egui::Context) {
    let mut show = app.show_settings;

    egui::Window::new("Settings")
        .open(&mut show)
        .resizable(false)
        .collapsible(false)
        .default_size([400.0, 460.0])
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::NONE
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(RADIUS + 2))
                .inner_margin(Margin::same(20)),
        )
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                ui.label(RichText::new("⚙").size(14.0).color(TEXT_SECONDARY));
                ui.label(
                    RichText::new("Settings")
                        .size(16.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
            });
            ui.label(
                RichText::new("How Lanker identifies & stores files")
                    .size(11.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(18.0);

            // Device name
            settings_field(ui, "🖥", "Device name", |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.config.device_name)
                        .font(FontId::proportional(13.0))
                        .hint_text("My PC"),
                );
            });

            // Port
            settings_field(ui, "🔌", "Port", |ui| {
                ui.add_enabled_ui(false, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut app.config.port)
                            .range(1024..=65535)
                            .speed(1),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new("the port devices connect to")
                        .size(10.0)
                        .color(TEXT_DIM),
                );
            });

            // Download folder
            settings_field(ui, "📁", "Download folder", |ui| {
                let dir_str = app.config.download_dir.display().to_string();
                let truncated = if dir_str.len() > 34 {
                    format!("..{}", &dir_str[dir_str.len() - 30..])
                } else {
                    dir_str
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(truncated).size(11.5).color(TEXT_SECONDARY));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let browse = ui.add(
                            egui::Button::new(
                                RichText::new("Browse").size(11.0).color(TEXT_SECONDARY),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(0.5_f32, BORDER))
                            .corner_radius(CornerRadius::same(6)),
                        );
                        if browse.clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                app.config.download_dir = dir;
                            }
                        }
                    });
                });
            });

            // Chunk size
            settings_field(ui, "📦", "Chunk size", |ui| {
                let mut chunk_mb = app.config.chunk_size / (1024 * 1024);
                ui.add(
                    egui::DragValue::new(&mut chunk_mb)
                        .range(1..=64)
                        .speed(1)
                        .suffix(" MB"),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("data sent per network packet")
                        .size(10.0)
                        .color(TEXT_DIM),
                );
                app.config.chunk_size = chunk_mb * 1024 * 1024;
            });

            ui.add_space(16.0);

            // Save
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let save_btn = ui.add(
                        egui::Button::new(
                            RichText::new("Save changes")
                                .size(12.5)
                                .strong()
                                .color(ACCENT_HOVER),
                        )
                        .fill(ACCENT.gamma_multiply(0.12))
                        .stroke(Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.55)))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(Vec2::new(120.0, 30.0)),
                    );
                    if save_btn.clicked() {
                        match app.config.save() {
                            Ok(()) => {
                                let mut st = app.transfer_state.lock().unwrap();
                                st.add_log("Settings saved".into());
                            }
                            Err(e) => {
                                let mut st = app.transfer_state.lock().unwrap();
                                st.add_log(format!("Failed to save settings: {e}"));
                            }
                        }
                        app.show_settings = false;
                    }
                });
            });
        });

    app.show_settings = show;
}

// ─── Incoming request ───────────────────────────────────────────────────────

fn render_incoming_request(app: &mut LankerApp, ctx: &egui::Context) {
    let request = {
        let ir = app.incoming_request.lock().unwrap();
        ir.clone()
    };

    let request = match request {
        Some(r) => r,
        None => return,
    };

    // Pop-in animation: fade the dim overlay, slide the card up, grow the icon ring.
    let now = ctx.input(|i| i.time);
    let anim_key = egui::Id::new(("incoming_pop", request.request_id));
    let start = ctx
        .data(|d| d.get_temp::<f64>(anim_key))
        .unwrap_or(now);
    ctx.data_mut(|d| {
        d.insert_temp(anim_key, start);
    });
    let t = (((now - start) as f32) / 0.28).clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - t).powi(3);
    if ease < 1.0 {
        ctx.request_repaint();
    }

    // Dim overlay
    let screen_rect = ctx.screen_rect();
    let slide = (1.0 - ease) * 16.0;
    egui::Area::new(egui::Id::new("incoming_overlay"))
        .fixed_pos(screen_rect.min)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen_rect.size(), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, Color32::from_black_alpha((150.0 * ease) as u8));
        });

    egui::Area::new(egui::Id::new("incoming_card"))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, slide))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.6)))
                .corner_radius(CornerRadius::same(16))
                .inner_margin(Margin::same(22))
                .show(ui, |ui| {
            ui.set_min_width(420.0);
            ui.set_max_width(420.0);
            ui.vertical_centered(|ui| {
                // icon in soft circle (grows as the card pops in)
                let icon_r = 16.0 + 11.0 * ease;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(icon_r * 2.0, icon_r * 2.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), icon_r, ACCENT.gamma_multiply(0.14 + 0.10 * ease));
                ui.painter().circle_stroke(rect.center(), icon_r, Stroke::new(1.5_f32, ACCENT.gamma_multiply(0.5 + 0.5 * ease)));
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "📨",
                    FontId::proportional(26.0),
                    Color32::WHITE,
                );
                ui.add_space(8.0);

                ui.label(
                    RichText::new("Incoming transfer")
                        .size(19.0)
                        .color(TEXT_PRIMARY)
                        .strong(),
                );
                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(
                        RichText::new(format!("From "))
                            .size(13.0)
                            .color(TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new(&request.sender_name)
                            .size(13.0)
                            .color(TEXT_PRIMARY)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!("({})", request.sender_ip))
                            .size(12.0)
                            .color(TEXT_DIM),
                    );
                });

                ui.add_space(6.0);

                // Summary pill
                let frame = egui::Frame::NONE
                    .fill(ACCENT.gamma_multiply(0.10))
                    .corner_radius(CornerRadius::same(255))
                    .inner_margin(Margin::symmetric(12, 5));
                frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} file{}  ·  {}",
                            request.files.len(),
                            if request.files.len() == 1 { "" } else { "s" },
                            format_bytes(request.total_size)
                        ))
                        .size(12.0)
                        .color(ACCENT_HOVER)
                        .strong(),
                    );
                });
            });

            ui.add_space(14.0);

            // File list
            let frame = egui::Frame::NONE
                .fill(SURFACE)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(10));
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for file in &request.files {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
                                ui.painter().rect_filled(rect, CornerRadius::same(4), ACCENT.gamma_multiply(0.12));
                                ui.painter().text(rect.center(), Align2::CENTER_CENTER, "📄", FontId::proportional(10.0), ACCENT_HOVER);
                                ui.label(
                                    RichText::new(&file.name)
                                        .size(12.0)
                                        .color(TEXT_PRIMARY),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format_bytes(file.size))
                                            .size(11.0)
                                            .color(TEXT_DIM),
                                    );
                                });
                            });
                            ui.add_space(3.0);
                        }
                    });
            });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                let button_width = (ui.available_width() - 12.0) / 2.0;

                let decline_btn = ui.add_sized(
                    [button_width, 40.0],
                    egui::Button::new(
                        RichText::new("Decline").size(14.0).strong().color(Color32::WHITE),
                    )
                    .fill(ERROR)
                    .stroke(Stroke::new(1.0_f32, ERROR))
                    .corner_radius(CornerRadius::same(10)),
                );
                if decline_btn.clicked() {
                    app.respond_to_incoming(false);
                }

                ui.add_space(12.0);

                let accept_btn = ui.add_sized(
                    [button_width, 40.0],
                    egui::Button::new(
                        RichText::new("✅  Accept").size(14.0).strong().color(Color32::WHITE),
                    )
                    .fill(ACCENT)
                    .stroke(Stroke::new(1.0_f32, ACCENT_HOVER))
                    .corner_radius(CornerRadius::same(10)),
                );
                if accept_btn.clicked() {
                    app.respond_to_incoming(true);
                }
            });
            });
        });
}

// ─── Drag & drop ────────────────────────────────────────────────────────────

fn handle_dropped_files(app: &mut LankerApp, ctx: &egui::Context) {
    let dropped_files: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
    for file in dropped_files {
        if let Some(path) = file.path {
            if let Some(pf) = PendingFile::from_path(path) {
                app.pending_files.push(pf);
            }
        }
    }
}
