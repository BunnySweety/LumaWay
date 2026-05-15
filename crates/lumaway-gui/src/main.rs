use adw::prelude::*;
use gtk::glib;
use lumaway_core::{
    lumaway_main_env_path, migrate_lumaway_env_v1, sync_mode::resolve_sync_mode, SyncMode,
    CONFIG_VERSION_KEY, CURRENT_CONFIG_VERSION, LEGACY_PRESET_KEY, SYNC_MODE_KEY,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;

mod i18n;

const APP_ID: &str = "io.github.BunnySweety.LumaWay";
const DEFAULT_SYNC_MODE: SyncMode = SyncMode::Video;
const COLOR_PROFILES: [&str; 6] = ["soft", "vivid", "game", "boosted", "cinema", "desktop"];
/// Shown in `connection_status` when the Hue bridge is reachable (keep in sync with comparisons).
const BRIDGE_STATUS_CONNECTED: &str = "Connected to bridge";
/// Zone-card dim labels: offset to match DropDown / Switch / scale trough (GTK aligns widget tops, not optical centers).
const ZONE_CARD_FIELD_LABEL_MARGIN_TOP: i32 = 8;

fn main() -> glib::ExitCode {
    if let Err(error) = migrate_lumaway_env_v1(&lumaway_main_env_path()) {
        eprintln!("lumaway-gui: failed to migrate config: {error}");
    }
    i18n::init_i18n();
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

#[derive(Clone)]
struct Ui {
    window: adw::ApplicationWindow,
    bridge: gtk::Entry,
    area: gtk::Entry,
    area_select: gtk::DropDown,
    area_enabled: gtk::Switch,
    area_lights_subtitle: gtk::Label,
    area_model: gtk::StringList,
    area_options: Rc<RefCell<Vec<AreaOption>>>,
    suppress_area_toggle: Rc<Cell<bool>>,
    app_key: gtk::PasswordEntry,
    client_key: gtk::PasswordEntry,
    duration: gtk::SpinButton,
    intensity: gtk::Scale,
    reactivity: gtk::Scale,
    profile: gtk::Entry,
    color_profile: gtk::DropDown,
    sync_mode: Rc<Cell<SyncMode>>,
    autostart: gtk::CheckButton,
    connection_status: gtk::Label,
    bridge_display: gtk::Label,
    settings: gtk::Button,
    discover: gtk::Button,
    auth: gtk::Button,
    start: gtk::Button,
    logs: gtk::TextBuffer,
    event_queue: Arc<Mutex<VecDeque<GuiEvent>>>,
    bridge_id: Rc<RefCell<String>>,
    /// Live label in the open Settings window; cleared on destroy.
    settings_bridge_id_label: Rc<RefCell<Option<gtk::Label>>>,
}

struct AppState {
    child: Option<Child>,
    log_queue: Arc<Mutex<VecDeque<String>>>,
    event_queue: Arc<Mutex<VecDeque<GuiEvent>>>,
    auto_quit_after_sync: bool,
    echo_logs: bool,
    pending_restart: bool,
}

enum GuiEvent {
    BridgesDiscovered(Vec<BridgeOption>),
    BridgeInfoLoaded { id: String, name: String },
    BridgeInfoUnavailable { message: String },
    AuthCreated { app_key: String, client_key: String },
    AreasLoaded(Vec<AreaOption>),
    AreaActivated { name: String, lights: usize },
    AreaActivateUnavailable { message: String },
    AreaDeactivated { name: String, lights: usize },
    AreaDeactivateUnavailable { message: String },
    ProfilesListed(Vec<String>),
    ProfileCalibrated { profile: String, output: String },
    CaptureQualityMeasured(String),
    Error(String),
}

#[derive(Debug, Clone)]
struct BridgeOption {
    ip: String,
    id: String,
}

#[derive(Debug, Clone)]
struct AreaOption {
    id: String,
    name: String,
    /// Light count for the Hue zone (from `list-areas`).
    lights: Option<usize>,
}

fn build_ui(app: &adw::Application) {
    install_css();
    let state = Rc::new(RefCell::new(AppState {
        child: None,
        log_queue: Arc::new(Mutex::new(VecDeque::new())),
        event_queue: Arc::new(Mutex::new(VecDeque::new())),
        auto_quit_after_sync: env_flag("LUMAWAY_GUI_QUIT_AFTER_SYNC"),
        echo_logs: env_flag("LUMAWAY_GUI_ECHO_LOGS"),
        pending_restart: false,
    }));

    let saved = read_env_file().unwrap_or_default();
    let event_queue = state.borrow().event_queue.clone();
    let ui = build_widgets(app, &saved, event_queue);
    wire_actions(&ui, state.clone());
    start_log_pump(&ui, state);
    ui.window.present();
    update_health(&ui);
    update_zone_controls(&ui, false);
    update_area_lights_subtitle(&ui);
    if ui.bridge.text().trim().is_empty() {
        discover_bridges(&ui);
    } else if ui_can_load_areas(&ui) {
        load_areas(&ui);
    }
    if env_flag("LUMAWAY_GUI_AUTOSTART") || ui.autostart.is_active() {
        ui.start.emit_clicked();
    }
}

fn build_widgets(
    app: &adw::Application,
    saved: &HashMap<String, String>,
    event_queue: Arc<Mutex<VecDeque<GuiEvent>>>,
) -> Ui {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("LumaWay")
        .default_width(460)
        .default_height(760)
        .build();
    window.add_css_class("lumaway-window");

    let toolbar = adw::HeaderBar::new();
    toolbar.add_css_class("flat");
    // Loose avoids symmetric start/end expansion (Strict + size group) that over-spreads CSD buttons.
    toolbar.set_centering_policy(adw::CenteringPolicy::Loose);
    let title = adw::WindowTitle::new("LumaWay", "Light sync");
    toolbar.set_title_widget(Some(&title));
    let settings = gtk::Button::with_label("Settings");
    settings.add_css_class("settings-button");

    let saved_bridge = saved.get("LUMAWAY_BRIDGE").cloned().unwrap_or_default();
    let saved_bridge_id = saved.get("LUMAWAY_BRIDGE_ID").cloned().unwrap_or_default();
    let saved_area = saved.get("LUMAWAY_AREA").cloned().unwrap_or_default();
    let bridge = gtk::Entry::builder()
        .text(&saved_bridge)
        .placeholder_text("Bridge IP address")
        .hexpand(true)
        .build();
    let area = gtk::Entry::builder()
        .text(&saved_area)
        .placeholder_text("Pick an entertainment zone")
        .hexpand(true)
        .build();
    let area_model = gtk::StringList::new(&[]);
    let mut initial_area_options = Vec::new();
    if !saved_area.is_empty() {
        area_model.append("Saved zone");
        initial_area_options.push(AreaOption {
            id: saved_area.clone(),
            name: "Saved zone".to_string(),
            lights: None,
        });
    }
    let area_select = gtk::DropDown::builder()
        .model(&area_model)
        .hexpand(true)
        .build();
    area_select.set_tooltip_text(Some("Zones loaded from the bridge"));
    let area_enabled = gtk::Switch::builder()
        .active(!saved_area.is_empty())
        .valign(gtk::Align::Center)
        .build();
    area_enabled.set_tooltip_text(Some("Turn the selected zone on or off"));
    let area_options = Rc::new(RefCell::new(initial_area_options));
    let suppress_area_toggle = Rc::new(Cell::new(false));
    let app_key = gtk::PasswordEntry::builder()
        .text(
            saved
                .get("LUMAWAY_APP_KEY")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .placeholder_text("Filled in automatically")
        .show_peek_icon(true)
        .hexpand(true)
        .build();
    let client_key = gtk::PasswordEntry::builder()
        .text(
            saved
                .get("LUMAWAY_CLIENT_KEY")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .placeholder_text("Filled in automatically")
        .show_peek_icon(true)
        .hexpand(true)
        .build();
    let duration = gtk::SpinButton::with_range(0.0, 86_400_000.0, 1000.0);
    duration.set_value(initial_duration_ms(saved) as f64);
    duration.set_numeric(true);
    duration.set_width_chars(10);
    duration.set_tooltip_text(Some("0 runs sync until you press Stop"));
    let intensity = percent_scale(initial_percent(saved, "LUMAWAY_BRIGHTNESS", 100.0));
    intensity.set_tooltip_text(Some("100 keeps captured brightness"));
    /* Value under the trough keeps the handle at the top of the widget; valign Start on the row aligns label with the track. */
    intensity.set_value_pos(gtk::PositionType::Bottom);
    let reactivity = percent_scale(initial_percent(saved, "LUMAWAY_REACTIVITY", 35.0));
    reactivity.set_tooltip_text(Some("Higher values react faster"));
    let profile = gtk::Entry::builder()
        .text(
            saved
                .get("LUMAWAY_PROFILE")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .placeholder_text("default")
        .hexpand(true)
        .build();
    profile.set_tooltip_text(Some("Profile file in ~/.config/lumaway/profiles"));
    let color_profile = color_profile_dropdown(
        saved
            .get("LUMAWAY_COLOR_PROFILE")
            .map(String::as_str)
            .unwrap_or("vivid"),
    );
    let sync_mode = Rc::new(Cell::new(initial_sync_mode(saved)));
    select_color_profile(&color_profile, color_profile_for_sync_mode(sync_mode.get()));
    let autostart = gtk::CheckButton::builder()
        .label("Start sync when app opens")
        .active(saved_flag(saved, "LUMAWAY_AUTOSTART_SYNC"))
        .build();
    autostart.set_tooltip_text(Some("Starts sync when the app launches"));

    let start = gtk::Button::with_label("Start sync");
    start.add_css_class("sync-control");
    start.add_css_class("suggested-action");
    start.add_css_class("primary-sync");
    start.set_halign(gtk::Align::Center);
    let auth = gtk::Button::with_label("Pair");
    let discover = gtk::Button::with_label("Discover");

    let connection_status = gtk::Label::new(Some("Bridge not configured"));
    connection_status.set_xalign(0.0);
    connection_status.add_css_class("connection-state");
    let bridge_display = gtk::Label::new(Some("Open Settings"));
    bridge_display.set_xalign(0.0);
    bridge_display.add_css_class("connection-title");
    let connection_copy = gtk::Box::new(gtk::Orientation::Vertical, 0);
    connection_copy.set_hexpand(true);
    connection_copy.append(&connection_status);
    connection_copy.append(&bridge_display);
    let connection_icon = gtk::Label::new(Some("◎"));
    connection_icon.add_css_class("connection-icon");
    let connection_header = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    connection_header.add_css_class("connection-header");
    connection_header.append(&connection_icon);
    connection_header.append(&connection_copy);
    connection_header.append(&settings);

    let area_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    area_box.add_css_class("view");
    area_box.add_css_class("zone-card");
    let zone_field_labels = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
    /* Single grid so column 0 is shared: "Zone" / "Brightness" stay left-aligned; avoids two grids sizing col0 differently. */
    let zone_card_grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .hexpand(true)
        .build();
    let zone_lbl = field_label("Zone");
    zone_field_labels.add_widget(&zone_lbl);
    zone_lbl.set_halign(gtk::Align::Start);
    zone_lbl.set_xalign(0.0);
    zone_lbl.set_valign(gtk::Align::Center);
    zone_card_grid.attach(&zone_lbl, 0, 0, 1, 1);
    area_select.set_valign(gtk::Align::Center);
    zone_card_grid.attach(&area_select, 1, 0, 1, 1);
    area_enabled.set_valign(gtk::Align::Center);
    area_enabled.set_hexpand(false);
    zone_card_grid.attach(&area_enabled, 2, 0, 1, 1);
    let area_lights_subtitle = gtk::Label::new(None);
    area_lights_subtitle.set_halign(gtk::Align::Start);
    area_lights_subtitle.set_xalign(0.0);
    area_lights_subtitle.add_css_class("area-lights-subtitle");
    zone_card_grid.attach(&area_lights_subtitle, 1, 1, 1, 1);
    let brightness_lbl = field_label("Brightness");
    zone_field_labels.add_widget(&brightness_lbl);
    brightness_lbl.set_halign(gtk::Align::Start);
    brightness_lbl.set_xalign(0.0);
    brightness_lbl.set_valign(gtk::Align::Start);
    brightness_lbl.set_margin_top(ZONE_CARD_FIELD_LABEL_MARGIN_TOP);
    zone_card_grid.attach(&brightness_lbl, 0, 2, 1, 1);
    intensity.set_valign(gtk::Align::Start);
    zone_card_grid.attach(&intensity, 1, 2, 1, 1);
    area_box.append(&zone_card_grid);

    let mode_box = section("Mode");
    mode_box.add_css_class("compact-card");
    mode_box.append(&mode_row(sync_mode.clone(), color_profile.clone()));
    let preset_tier = gtk::Box::new(gtk::Orientation::Vertical, 10);
    preset_tier.add_css_class("mode-presets-tier");
    let preset_heading = gtk::Label::new(Some("Reactivity"));
    preset_heading.set_xalign(0.0);
    preset_heading.add_css_class("mode-section-subtitle");
    preset_tier.append(&preset_heading);
    preset_tier.append(&preset_row());
    mode_box.append(&preset_tier);

    let sync_footer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sync_footer.set_hexpand(true);
    sync_footer.set_margin_top(16);
    sync_footer.set_margin_start(22);
    sync_footer.set_margin_end(22);
    sync_footer.set_margin_bottom(22);
    sync_footer.append(&start);

    let logs = gtk::TextBuffer::new(None);
    logs.set_text("Sync events will appear here.\n");

    let lower = gtk::Box::new(gtk::Orientation::Vertical, 10);
    lower.add_css_class("main-panel");
    lower.set_margin_top(22);
    lower.set_margin_start(22);
    lower.set_margin_end(22);
    lower.set_margin_bottom(8);
    lower.append(&connection_header);
    lower.append(&area_box);
    lower.append(&mode_box);
    let autostart_holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    autostart_holder.set_visible(false);
    autostart_holder.append(&autostart);
    lower.append(&autostart_holder);

    let page_scroll = gtk::ScrolledWindow::builder()
        .child(&lower)
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .build();

    let page_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page_root.set_vexpand(true);
    page_root.set_hexpand(true);
    page_root.append(&page_scroll);
    page_root.append(&sync_footer);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.set_top_bar_style(adw::ToolbarStyle::Flat);
    toolbar_view.add_top_bar(&toolbar);
    toolbar_view.set_content(Some(&page_root));
    toolbar_view.set_vexpand(true);
    toolbar_view.set_hexpand(true);

    window.set_content(Some(&toolbar_view));

    Ui {
        window,
        bridge,
        area,
        area_select,
        area_enabled,
        area_lights_subtitle,
        area_model,
        area_options,
        suppress_area_toggle,
        app_key,
        client_key,
        duration,
        intensity,
        reactivity,
        profile,
        color_profile,
        sync_mode,
        autostart,
        connection_status,
        bridge_display,
        settings,
        discover,
        auth,
        start,
        logs,
        event_queue,
        bridge_id: Rc::new(RefCell::new(saved_bridge_id)),
        settings_bridge_id_label: Rc::new(RefCell::new(None)),
    }
}

fn field_label(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(1.0);
    label.add_css_class("dim-label");
    label
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.connect_parsing_error(|_prov, _section, error| {
        eprintln!("lumaway-gui CSS: {error}");
    });
    provider.load_from_string(
        r#"
        window.lumaway-window {
            background: radial-gradient(circle at 50% 0%, #173954 0%, #101922 42%, #06080b 100%);
            color: #f4fbff;
        }
        headerbar {
            background: rgba(8, 14, 20, 0.72);
            color: #f4fbff;
            box-shadow: none;
        }
        .main-panel {
            background: transparent;
        }
        .hero-title {
            color: #f4fbff;
            font-size: 22px;
            font-weight: 700;
            margin-bottom: 6px;
        }
        .connection-header {
            padding: 16px 6px 12px 6px;
        }
        .connection-icon {
            min-width: 38px;
            min-height: 38px;
            color: #f4fbff;
            font-size: 28px;
            font-weight: 700;
        }
        .connection-state {
            color: rgba(146, 214, 60, 0.96);
            font-weight: 700;
            font-size: 15px;
        }
        .connection-state.warning {
            color: rgba(238, 188, 85, 0.96);
        }
        .connection-state.connected {
            color: rgba(146, 214, 60, 0.96);
        }
        .connection-title {
            color: #f4fbff;
            font-size: 20px;
            font-weight: 800;
        }
        .view {
            background: rgba(19, 31, 42, 0.72);
            border-radius: 18px;
            padding: 16px;
            box-shadow: 0 12px 28px rgba(0, 0, 0, 0.28);
        }
        .compact-card {
            border: 1px solid rgba(255, 255, 255, 0.07);
        }
        .zone-card {
            background: linear-gradient(135deg, #1680df 0%, #79caff 100%);
            color: #ffffff;
            border-radius: 18px;
            padding: 18px;
            box-shadow: 0 18px 34px rgba(12, 109, 204, 0.34);
        }
        .zone-card .dim-label {
            color: rgba(255, 255, 255, 0.98);
            font-weight: 600;
        }
        .zone-card switch {
            margin-top: 0;
        }
        .zone-card .area-lights-subtitle {
            color: rgba(255, 255, 255, 0.94);
            font-size: 13px;
            font-weight: 600;
            margin: 0;
            padding: 0;
            padding-left: 12px;
        }
        .heading {
            color: #f4fbff;
            font-weight: 700;
        }
        .dim-label {
            color: rgba(226, 239, 248, 0.68);
        }
        /* Do not style header CSD buttons globally — app-wide button padding used to spread them apart */
        .main-panel button,
        .connection-header button,
        window.settings-window button {
            border-radius: 999px;
            padding: 8px 16px;
            font-weight: 700;
        }
        /* Match padding-left to .area-lights-subtitle; GtkLabel has no chevron inset */
        .main-panel .zone-card dropdown > button {
            padding: 7px 12px 7px 12px;
            border-radius: 12px;
            font-weight: 600;
        }
        button.sync-control {
            min-width: 168px;
            min-height: 40px;
            padding: 7px 14px;
            font-size: 16px;
            font-weight: 700;
            box-shadow: none;
        }
        button.sync-control.primary-sync {
            background: linear-gradient(180deg, #5f8f52 0%, #3d6b38 100%);
            color: #f4fbf2;
        }
        button.secondary-sync {
            min-height: 48px;
            margin-top: 18px;
        }
        button.settings-button {
            background: rgba(255, 255, 255, 0.16);
            color: #f4fbff;
            border-radius: 999px;
            padding: 8px 18px;
        }
        button.mode-icon-button {
            min-width: 56px;
            min-height: 56px;
            padding: 0;
            border-radius: 999px;
            background: rgba(255, 255, 255, 0.12);
            color: rgba(244, 251, 255, 0.94);
            box-shadow: none;
        }
        button.mode-icon-button image {
            opacity: 0.98;
        }
        button.mode-icon-button.active-mode {
            background: rgba(255, 255, 255, 0.95);
            color: #101922;
        }
        .mode-tile-caption {
            color: rgba(226, 239, 248, 0.55);
            font-size: 12px;
            font-weight: 600;
        }
        .mode-tile-caption-active {
            color: rgba(244, 251, 255, 0.95);
            font-weight: 700;
        }
        .mode-presets-tier {
            margin-top: 4px;
            padding-top: 18px;
            border-top: 1px solid rgba(255, 255, 255, 0.1);
        }
        .mode-section-subtitle {
            color: rgba(226, 239, 248, 0.6);
            font-size: 12px;
            font-weight: 700;
            letter-spacing: 0.03em;
        }
        .preset-button {
            min-width: 78px;
            min-height: 40px;
            background: rgba(255, 255, 255, 0.12);
            color: rgba(244, 251, 255, 0.78);
            border-radius: 10px;
            font-weight: 600;
        }
        .preset-button.active-preset {
            background: rgba(255, 255, 255, 0.78);
            color: #24313d;
        }
        scale trough {
            min-height: 12px;
            border-radius: 999px;
            background: rgba(255, 255, 255, 0.18);
        }
        scale highlight {
            border-radius: 999px;
            background: linear-gradient(90deg, #d8f8ff 0%, #ffffff 100%);
        }
        scale slider {
            min-width: 28px;
            min-height: 28px;
            border-radius: 999px;
            background: #ffffff;
        }
        entry, passwordentry, spinbutton, dropdown {
            border-radius: 12px;
        }
        expander {
            color: rgba(244, 251, 255, 0.76);
        }
        textview {
            color: #dcebf4;
            background: rgba(3, 7, 10, 0.70);
        }
        window.settings-window {
            background: #0b1118;
            color: #f4fbff;
        }
        /*
         * CSD controls live in AdwHeaderBar .end GtkBox (not directly under headerbar).
         * border-spacing targets GtkWindowControls internal GtkBoxLayout (GTK 4.12+).
         */
        window.lumaway-window headerbar .end windowcontrols {
            border-spacing: 0px;
        }
        window.lumaway-window headerbar .end windowcontrols > button {
            padding: 0;
            margin: 0;
            min-width: 34px;
            min-height: 34px;
        }
        "#,
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}

fn section(title: &str) -> gtk::Box {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 10);
    container.add_css_class("view");
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    container.append(&label);
    container
}

struct ModeTile {
    mode: SyncMode,
    button: gtk::ToggleButton,
    caption: gtk::Label,
}

fn mode_row(selected_mode: Rc<Cell<SyncMode>>, color_profile: gtk::DropDown) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_homogeneous(true);
    row.set_margin_top(4);
    /* Freedesktop-style symbolic names (Adwaita, Breeze, etc.) — crisp at any DPI */
    const ICON_PX: i32 = 28;
    let tiles = Rc::new(RefCell::new(Vec::<ModeTile>::new()));
    for (mode, icon_name, label, enabled) in [
        (SyncMode::Video, "video-display-symbolic", "Video", true),
        (SyncMode::Game, "input-gaming-symbolic", "Game", true),
        (SyncMode::Desktop, "computer-symbolic", "Desktop", true),
        (SyncMode::Music, "audio-headphones-symbolic", "Music", false),
    ] {
        let active = mode == selected_mode.get();
        let tile = gtk::Box::new(gtk::Orientation::Vertical, 8);
        tile.set_hexpand(true);
        tile.set_halign(gtk::Align::Center);

        let image = gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(ICON_PX)
            .build();
        let circle = gtk::ToggleButton::builder().child(&image).build();
        circle.add_css_class("mode-icon-button");
        circle.set_active(active);
        circle.set_sensitive(enabled);
        if active {
            circle.add_css_class("active-mode");
        }
        if !enabled {
            circle.set_tooltip_text(Some(i18n::tr("Coming soon").as_str()));
            tile.set_tooltip_text(Some(i18n::tr("Coming soon").as_str()));
        }

        let caption = gtk::Label::new(Some(i18n::tr(label).as_str()));
        caption.set_xalign(0.5);
        caption.add_css_class("mode-tile-caption");
        if active {
            caption.add_css_class("mode-tile-caption-active");
        }

        tile.append(&circle);
        tile.append(&caption);
        row.append(&tile);
        let all_tiles = tiles.clone();
        let selected_mode = selected_mode.clone();
        let color_profile = color_profile.clone();
        circle.connect_clicked(move |_| {
            selected_mode.set(mode);
            select_color_profile(&color_profile, color_profile_for_sync_mode(mode));
            let all_tiles = all_tiles.borrow();
            refresh_mode_tiles(selected_mode.get(), all_tiles.as_slice());
        });
        tiles.borrow_mut().push(ModeTile {
            mode,
            button: circle,
            caption,
        });
    }
    let all_tiles = tiles.borrow();
    refresh_mode_tiles(selected_mode.get(), all_tiles.as_slice());
    row
}

fn refresh_mode_tiles(selected_mode: SyncMode, tiles: &[ModeTile]) {
    for tile in tiles {
        let active = tile.mode == selected_mode;
        tile.button.set_active(active);
        if active {
            tile.button.add_css_class("active-mode");
            tile.caption.add_css_class("mode-tile-caption-active");
        } else {
            tile.button.remove_css_class("active-mode");
            tile.caption.remove_css_class("mode-tile-caption-active");
        }
    }
}

fn preset_row() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_homogeneous(true);
    for (label, active) in [
        ("Subtle", false),
        ("Moderate", true),
        ("High", false),
        ("Intense", false),
    ] {
        let button = gtk::Button::with_label(label);
        button.add_css_class("preset-button");
        if active {
            button.add_css_class("active-preset");
        }
        row.append(&button);
    }
    row
}

fn percent_scale(value: f64) -> gtk::Scale {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_value(value.clamp(0.0, 100.0));
    scale.set_draw_value(true);
    scale.set_digits(0);
    scale.set_hexpand(true);
    scale
}

fn color_profile_dropdown(active: &str) -> gtk::DropDown {
    let model = gtk::StringList::new(&COLOR_PROFILES);
    let dropdown = gtk::DropDown::builder().model(&model).build();
    select_color_profile(&dropdown, active);
    dropdown.set_tooltip_text(Some("Color grading profile"));
    dropdown
}

fn select_color_profile(dropdown: &gtk::DropDown, active: &str) {
    let selected = COLOR_PROFILES
        .iter()
        .position(|profile| profile.eq_ignore_ascii_case(active.trim()))
        .unwrap_or(1);
    dropdown.set_selected(selected as u32);
}

fn selected_string(dropdown: &gtk::DropDown) -> String {
    dropdown
        .selected_item()
        .and_downcast::<gtk::StringObject>()
        .map(|item| item.string().to_string())
        .unwrap_or_else(|| "vivid".to_string())
}

fn sanitize_color_profile(value: &str) -> &str {
    match value.trim() {
        "soft" | "vivid" | "game" | "boosted" | "cinema" | "desktop" => value.trim(),
        _ => "vivid",
    }
}

fn row<W>(label: &str, widget: &W) -> gtk::Grid
where
    W: IsA<gtk::Widget>,
{
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(4)
        .build();
    grid.attach(&field_label(label), 0, 0, 1, 1);
    grid.attach(widget, 1, 0, 1, 1);
    grid
}

fn wire_actions(ui: &Ui, state: Rc<RefCell<AppState>>) {
    let settings_ui = ui.clone();
    ui.settings.connect_clicked(move |_| {
        open_settings_window(&settings_ui);
    });

    let discover_ui = ui.clone();
    ui.discover.connect_clicked(move |_| {
        discover_bridges(&discover_ui);
    });

    let auth_ui = ui.clone();
    ui.auth.connect_clicked(move |_| {
        create_hue_keys(&auth_ui);
    });

    let area_ui = ui.clone();
    let area_entry = ui.area.clone();
    let area_options = ui.area_options.clone();
    let area_state = state.clone();
    ui.area_select.connect_selected_notify(move |select| {
        let index = select.selected() as usize;
        if let Some(area) = area_options.borrow().get(index) {
            area_entry.set_text(&area.id);
            update_area_lights_subtitle(&area_ui);
            if area_ui.suppress_area_toggle.get() {
                return;
            }
            let _ = write_current_env_file(&area_ui);
            update_zone_controls(&area_ui, sync_running(&area_state));
            if area_ui.area_enabled.is_active() && !sync_running(&area_state) {
                activate_area(&area_ui);
            }
        }
    });

    let zone_ui = ui.clone();
    let zone_state = state.clone();
    ui.area_enabled.connect_active_notify(move |_| {
        if zone_ui.suppress_area_toggle.get() {
            return;
        }
        let running = sync_running(&zone_state);
        let _ = write_current_env_file(&zone_ui);
        update_zone_controls(&zone_ui, running);
        if zone_ui.area_enabled.is_active() {
            activate_area(&zone_ui);
        } else if running {
            zone_state.borrow_mut().pending_restart = false;
            stop_sync(&zone_ui, &zone_state);
            deactivate_area(&zone_ui);
        } else {
            deactivate_area(&zone_ui);
        }
    });

    let intensity_ui = ui.clone();
    let intensity_state = state.clone();
    ui.intensity.connect_value_changed(move |_| {
        let _ = write_current_env_file(&intensity_ui);
        if sync_running(&intensity_state) {
            schedule_sync_restart(&intensity_ui, &intensity_state);
        } else if intensity_ui.area_enabled.is_active()
            && !intensity_ui.area.text().trim().is_empty()
        {
            activate_area(&intensity_ui);
        }
    });

    let start_ui = ui.clone();
    let start_state = state.clone();
    ui.start.connect_clicked(move |_| {
        if sync_running(&start_state) {
            stop_sync(&start_ui, &start_state);
        } else if let Err(error) = start_sync(&start_ui, &start_state) {
            append_log(&start_ui.logs, &format!("error: {error}\n"));
            apply_sync_idle_button_style(&start_ui);
            update_zone_controls(&start_ui, false);
        }
    });

    let close_ui = ui.clone();
    let close_state = state.clone();
    ui.window.connect_close_request(move |_| {
        stop_sync(&close_ui, &close_state);
        glib::Propagation::Proceed
    });
}

fn open_settings_window(ui: &Ui) {
    let window = gtk::Window::builder()
        .title("Settings")
        .default_width(440)
        .default_height(680)
        .transient_for(&ui.window)
        .modal(false)
        .build();
    window.add_css_class("settings-window");

    let bridge = gtk::Entry::builder()
        .text(ui.bridge.text().as_str())
        .placeholder_text("Bridge IP address")
        .hexpand(true)
        .build();
    let area = gtk::Entry::builder()
        .text(ui.area.text().as_str())
        .placeholder_text("Zone ID or name")
        .hexpand(true)
        .build();
    let app_key = gtk::PasswordEntry::builder()
        .text(ui.app_key.text().as_str())
        .placeholder_text("Application key")
        .show_peek_icon(true)
        .hexpand(true)
        .build();
    let client_key = gtk::PasswordEntry::builder()
        .text(ui.client_key.text().as_str())
        .placeholder_text("Streaming key")
        .show_peek_icon(true)
        .hexpand(true)
        .build();
    let duration = gtk::SpinButton::with_range(0.0, 86_400_000.0, 1000.0);
    duration.set_value(ui.duration.value());
    duration.set_numeric(true);
    let intensity = percent_scale(ui.intensity.value());
    let reactivity = percent_scale(ui.reactivity.value());
    let profile = gtk::Entry::builder()
        .text(ui.profile.text().as_str())
        .placeholder_text("default")
        .hexpand(true)
        .build();
    profile.set_tooltip_text(Some("Profile file in ~/.config/lumaway/profiles"));
    let color_profile = color_profile_dropdown(&selected_string(&ui.color_profile));
    let autostart = gtk::CheckButton::builder()
        .label("Start sync when app opens")
        .active(ui.autostart.is_active())
        .build();

    let bridge_id_label = gtk::Label::new(Some(
        bridge_id_display_text(ui.bridge_id.borrow().as_str()).as_str(),
    ));
    bridge_id_label.set_xalign(0.0);
    bridge_id_label.add_css_class("dim-label");
    bridge_id_label.set_wrap(true);
    bridge_id_label.set_selectable(true);

    let connection = section("Connection");
    connection.add_css_class("compact-card");
    connection.append(&row("Bridge address", &bridge));
    connection.append(&row("Bridge hardware id", &bridge_id_label));
    connection.append(&row("Zone ID", &area));
    connection.append(&row("Application key", &app_key));
    connection.append(&row("Streaming key", &client_key));

    let tuning = section("Sync");
    tuning.add_css_class("compact-card");
    tuning.append(&row("Brightness", &intensity));
    tuning.append(&row("Reactivity", &reactivity));
    tuning.append(&row("Capture profile", &profile));
    tuning.append(&row("Color profile", &color_profile));
    tuning.append(&row("Duration (ms)", &duration));
    tuning.append(&autostart);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let detect = gtk::Button::with_label("Discover");
    let pair = gtk::Button::with_label("Pair");
    let profiles = gtk::Button::with_label("Profiles");
    profiles.set_tooltip_text(Some("List profiles from ~/.config/lumaway/profiles"));
    let quality = gtk::Button::with_label("Quality");
    quality.set_tooltip_text(Some("Measure capture quality for the configured zone"));
    let calibrate = gtk::Button::with_label("Calibrate");
    calibrate.set_tooltip_text(Some(
        "Probe capture backends and write the selected profile",
    ));
    let save = gtk::Button::with_label("Save");
    save.add_css_class("suggested-action");
    actions.append(&detect);
    actions.append(&pair);
    actions.append(&profiles);
    actions.append(&quality);
    actions.append(&calibrate);
    actions.append(&save);

    let logs = section("Log");
    logs.add_css_class("compact-card");
    let log_view = gtk::TextView::builder()
        .buffer(&ui.logs)
        .editable(false)
        .monospace(true)
        .vexpand(true)
        .build();
    let log_scroll = gtk::ScrolledWindow::builder()
        .child(&log_view)
        .min_content_height(170)
        .vexpand(true)
        .build();
    logs.append(&log_scroll);

    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_top(18);
    body.set_margin_bottom(18);
    body.set_margin_start(18);
    body.set_margin_end(18);
    body.append(&connection);
    body.append(&tuning);
    body.append(&actions);
    body.append(&logs);

    let scrolled = gtk::ScrolledWindow::builder()
        .child(&body)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    window.set_child(Some(&scrolled));

    let save_ui = ui.clone();
    let save_bridge = bridge.clone();
    let save_area = area.clone();
    let save_app_key = app_key.clone();
    let save_client_key = client_key.clone();
    let save_duration = duration.clone();
    let save_intensity = intensity.clone();
    let save_reactivity = reactivity.clone();
    let save_profile = profile.clone();
    let save_color_profile = color_profile.clone();
    let save_autostart = autostart.clone();
    save.connect_clicked(move |_| {
        apply_settings_values(
            &save_ui,
            SettingsControls {
                bridge: &save_bridge,
                area: &save_area,
                app_key: &save_app_key,
                client_key: &save_client_key,
                duration: &save_duration,
                intensity: &save_intensity,
                reactivity: &save_reactivity,
                profile: &save_profile,
                color_profile: &save_color_profile,
                autostart: &save_autostart,
            },
        );
        if ui_can_load_areas(&save_ui) {
            load_areas(&save_ui);
        }
    });

    let detect_ui = ui.clone();
    let detect_bridge = bridge.clone();
    let detect_area = area.clone();
    let detect_app_key = app_key.clone();
    let detect_client_key = client_key.clone();
    let detect_duration = duration.clone();
    let detect_intensity = intensity.clone();
    let detect_reactivity = reactivity.clone();
    let detect_profile = profile.clone();
    let detect_color_profile = color_profile.clone();
    let detect_autostart = autostart.clone();
    detect.connect_clicked(move |_| {
        apply_settings_values(
            &detect_ui,
            SettingsControls {
                bridge: &detect_bridge,
                area: &detect_area,
                app_key: &detect_app_key,
                client_key: &detect_client_key,
                duration: &detect_duration,
                intensity: &detect_intensity,
                reactivity: &detect_reactivity,
                profile: &detect_profile,
                color_profile: &detect_color_profile,
                autostart: &detect_autostart,
            },
        );
        discover_bridges(&detect_ui);
    });

    let pair_ui = ui.clone();
    let pair_bridge = bridge.clone();
    let pair_area = area.clone();
    let pair_app_key = app_key.clone();
    let pair_client_key = client_key.clone();
    let pair_duration = duration.clone();
    let pair_intensity = intensity.clone();
    let pair_reactivity = reactivity.clone();
    let pair_profile = profile.clone();
    let pair_color_profile = color_profile.clone();
    let pair_autostart = autostart.clone();
    pair.connect_clicked(move |_| {
        apply_settings_values(
            &pair_ui,
            SettingsControls {
                bridge: &pair_bridge,
                area: &pair_area,
                app_key: &pair_app_key,
                client_key: &pair_client_key,
                duration: &pair_duration,
                intensity: &pair_intensity,
                reactivity: &pair_reactivity,
                profile: &pair_profile,
                color_profile: &pair_color_profile,
                autostart: &pair_autostart,
            },
        );
        create_hue_keys(&pair_ui);
    });

    let profiles_ui = ui.clone();
    profiles.connect_clicked(move |_| {
        list_capture_profiles(&profiles_ui);
    });

    let quality_ui = ui.clone();
    let quality_bridge = bridge.clone();
    let quality_area = area.clone();
    let quality_app_key = app_key.clone();
    let quality_client_key = client_key.clone();
    let quality_duration = duration.clone();
    let quality_intensity = intensity.clone();
    let quality_reactivity = reactivity.clone();
    let quality_profile = profile.clone();
    let quality_color_profile = color_profile.clone();
    let quality_autostart = autostart.clone();
    quality.connect_clicked(move |_| {
        apply_settings_values(
            &quality_ui,
            SettingsControls {
                bridge: &quality_bridge,
                area: &quality_area,
                app_key: &quality_app_key,
                client_key: &quality_client_key,
                duration: &quality_duration,
                intensity: &quality_intensity,
                reactivity: &quality_reactivity,
                profile: &quality_profile,
                color_profile: &quality_color_profile,
                autostart: &quality_autostart,
            },
        );
        measure_capture_quality(&quality_ui);
    });

    let calibrate_ui = ui.clone();
    let calibrate_bridge = bridge.clone();
    let calibrate_area = area.clone();
    let calibrate_app_key = app_key.clone();
    let calibrate_client_key = client_key.clone();
    let calibrate_duration = duration.clone();
    let calibrate_intensity = intensity.clone();
    let calibrate_reactivity = reactivity.clone();
    let calibrate_profile = profile.clone();
    let calibrate_color_profile = color_profile.clone();
    let calibrate_autostart = autostart.clone();
    calibrate.connect_clicked(move |_| {
        apply_settings_values(
            &calibrate_ui,
            SettingsControls {
                bridge: &calibrate_bridge,
                area: &calibrate_area,
                app_key: &calibrate_app_key,
                client_key: &calibrate_client_key,
                duration: &calibrate_duration,
                intensity: &calibrate_intensity,
                reactivity: &calibrate_reactivity,
                profile: &calibrate_profile,
                color_profile: &calibrate_color_profile,
                autostart: &calibrate_autostart,
            },
        );
        calibrate_capture_profile(&calibrate_ui);
    });

    let settings_ui = ui.clone();
    let settings_label_slot = ui.settings_bridge_id_label.clone();
    *settings_label_slot.borrow_mut() = Some(bridge_id_label.clone());
    window.connect_destroy(move |_| {
        *settings_ui.settings_bridge_id_label.borrow_mut() = None;
    });

    window.present();
}

fn bridge_id_display_text(bridge_id: &str) -> String {
    let bridge_id = bridge_id.trim();
    if bridge_id.is_empty() {
        "— (load zones after pairing)".to_string()
    } else {
        bridge_id.to_string()
    }
}

fn refresh_bridge_id_displays(ui: &Ui) {
    let text = bridge_id_display_text(ui.bridge_id.borrow().as_str());
    if let Some(label) = ui.settings_bridge_id_label.borrow().as_ref() {
        label.set_text(&text);
    }
}

struct SettingsControls<'a> {
    bridge: &'a gtk::Entry,
    area: &'a gtk::Entry,
    app_key: &'a gtk::PasswordEntry,
    client_key: &'a gtk::PasswordEntry,
    duration: &'a gtk::SpinButton,
    intensity: &'a gtk::Scale,
    reactivity: &'a gtk::Scale,
    profile: &'a gtk::Entry,
    color_profile: &'a gtk::DropDown,
    autostart: &'a gtk::CheckButton,
}

fn apply_settings_values(ui: &Ui, controls: SettingsControls<'_>) {
    ui.bridge.set_text(controls.bridge.text().trim());
    ui.area.set_text(controls.area.text().trim());
    ui.app_key.set_text(controls.app_key.text().as_str());
    ui.client_key.set_text(controls.client_key.text().as_str());
    ui.duration.set_value(controls.duration.value());
    ui.intensity.set_value(controls.intensity.value());
    ui.reactivity.set_value(controls.reactivity.value());
    ui.profile.set_text(controls.profile.text().trim());
    ui.color_profile
        .set_selected(controls.color_profile.selected());
    ui.autostart.set_active(controls.autostart.is_active());
    update_health(ui);

    let _ = write_env_file(env_file_values_from_ui(ui, ui.area.text().trim()));
    append_log(&ui.logs, "Settings saved.\n");
}

fn discover_bridges(ui: &Ui) {
    set_connection_header(ui, "Searching bridge", "Please wait…", "warning");
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("discover-bridges")
            .output();
        let event = match output {
            Ok(output) if output.status.success() => parse_bridge_discovery_output(&output.stdout)
                .map(GuiEvent::BridgesDiscovered)
                .unwrap_or_else(GuiEvent::Error),
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn create_hue_keys(ui: &Ui) {
    let bridge = ui.bridge.text().trim().to_string();
    if bridge.is_empty() {
        append_log(
            &ui.logs,
            "error: bridge address is required before pairing\n",
        );
        return;
    }

    if ui.app_key.text().is_empty() && ui.client_key.text().is_empty() {
        append_log(
            &ui.logs,
            "Press the physical button on the bridge, then wait.\n",
        );
    } else {
        append_log(
            &ui.logs,
            "Press the physical button on the bridge, then wait. New pairing keys will replace the saved keys.\n",
        );
    }
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("auth")
            .arg("--bridge")
            .arg(&bridge)
            .output();
        let event = match output {
            Ok(output) if output.status.success() => parse_auth_output(&output.stdout)
                .map(|(app_key, client_key)| GuiEvent::AuthCreated {
                    app_key,
                    client_key,
                })
                .unwrap_or_else(GuiEvent::Error),
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn calibrate_capture_profile(ui: &Ui) {
    let profile = sanitize_profile_name(ui.profile.text().as_str());
    if profile.is_empty() {
        append_log(&ui.logs, "error: capture profile name is required\n");
        return;
    }

    append_log(
        &ui.logs,
        &format!("Calibrating capture profile `{profile}`. Select the display in Portal.\n"),
    );
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("calibrate-capture")
            .arg("--name")
            .arg(&profile)
            .arg("--force")
            .output();
        let event = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                GuiEvent::ProfileCalibrated {
                    profile,
                    output: format!("{stderr}{stdout}"),
                }
            }
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn list_capture_profiles(ui: &Ui) {
    append_log(&ui.logs, "Listing capture profiles.\n");
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("profile-list")
            .output();
        let event = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                GuiEvent::ProfilesListed(parse_profile_list_output(&stdout))
            }
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn measure_capture_quality(ui: &Ui) {
    if ui.area.text().trim().is_empty() {
        append_log(
            &ui.logs,
            "error: zone is required before measuring capture quality\n",
        );
        return;
    }

    append_log(
        &ui.logs,
        "Measuring capture quality. Select the display in Portal.\n",
    );
    let sync_mode = screen_sync_mode(ui.sync_mode.get());
    let preset = preset_for_sync_mode(sync_mode);
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("capture-quality")
            .arg("--portal")
            .arg("--sync-mode")
            .arg(sync_mode.as_env_value())
            .arg("--preset")
            .arg(preset)
            .arg("--frames")
            .arg("30")
            .output();
        let event = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                GuiEvent::CaptureQualityMeasured(stdout.trim().to_string())
            }
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn load_areas(ui: &Ui) {
    let bridge = ui.bridge.text().trim().to_string();
    let app_key = ui.app_key.text().to_string();
    if bridge.is_empty() || app_key.is_empty() {
        append_log(
            &ui.logs,
            "error: bridge address and application key are required before loading zones\n",
        );
        return;
    }

    load_bridge_info(ui);
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("list-areas")
            .arg("--bridge")
            .arg(&bridge)
            .arg("--app-key")
            .arg(&app_key)
            .output();
        let event = match output {
            Ok(output) if output.status.success() => parse_areas_output(&output.stdout)
                .map(GuiEvent::AreasLoaded)
                .unwrap_or_else(GuiEvent::Error),
            Ok(output) => GuiEvent::Error(String::from_utf8_lossy(&output.stderr).to_string()),
            Err(error) => GuiEvent::Error(error.to_string()),
        };
        push_event(&queue, event);
    });
}

fn load_bridge_info(ui: &Ui) {
    let bridge = ui.bridge.text().trim().to_string();
    let app_key = ui.app_key.text().trim().to_string();
    if bridge.is_empty() || app_key.is_empty() {
        return;
    }

    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let output = Command::new(resolve_lumaway_binary())
            .arg("bridge-info")
            .arg("--bridge")
            .arg(&bridge)
            .arg("--app-key")
            .arg(&app_key)
            .output();
        match output {
            Ok(output) if output.status.success() => {
                if let Ok(info) = parse_bridge_info_output(&output.stdout) {
                    push_event(
                        &queue,
                        GuiEvent::BridgeInfoLoaded {
                            id: info.id,
                            name: info.name,
                        },
                    );
                }
            }
            Ok(output) => push_event(
                &queue,
                GuiEvent::BridgeInfoUnavailable {
                    message: String::from_utf8_lossy(&output.stderr).to_string(),
                },
            ),
            Err(error) => push_event(
                &queue,
                GuiEvent::BridgeInfoUnavailable {
                    message: error.to_string(),
                },
            ),
        }
    });
}

fn activate_area(ui: &Ui) {
    set_area_active(ui, true, Some(ui.intensity.value()));
}

fn deactivate_area(ui: &Ui) {
    set_area_active(ui, false, None);
}

fn set_area_active(ui: &Ui, active: bool, brightness: Option<f64>) {
    let bridge = ui.bridge.text().trim().to_string();
    let area = current_area_ref(ui);
    ui.area.set_text(&area);
    let app_key = ui.app_key.text().trim().to_string();
    if bridge.is_empty() || area.is_empty() || app_key.is_empty() {
        append_log(
            &ui.logs,
            "error: zone, bridge address, or application key missing\n",
        );
        return;
    }

    append_log(
        &ui.logs,
        &format!(
            "{} zone={} level={}\n",
            if active { "Activate" } else { "Deactivate" },
            area,
            brightness
                .map(|value| format!("{value:.0}%"))
                .unwrap_or_else(|| "n/a".to_string())
        ),
    );
    let queue = ui_event_queue(ui);
    thread::spawn(move || {
        let mut command = Command::new(resolve_lumaway_binary());
        command
            .arg(if active {
                "activate-area"
            } else {
                "deactivate-area"
            })
            .arg("--bridge")
            .arg(&bridge)
            .arg("--app-key")
            .arg(&app_key)
            .arg("--area")
            .arg(&area);
        let output = if let Some(brightness) = brightness {
            // activate-area expects a Hue percentage in 1..=100, not a 0..=1 fraction.
            let pct = brightness.clamp(0.0, 100.0).max(1.0);
            command
                .arg("--brightness")
                .arg(format!("{pct:.2}"))
                .output()
        } else {
            command.output()
        };
        match output {
            Ok(output) if output.status.success() => {
                let parsed = parse_area_state_output(&output.stdout);
                let event = match (active, parsed) {
                    (true, Ok((name, lights))) => GuiEvent::AreaActivated { name, lights },
                    (false, Ok((name, lights))) => GuiEvent::AreaDeactivated { name, lights },
                    (_, Err(message)) => GuiEvent::AreaActivateUnavailable { message },
                };
                push_event(&queue, event);
            }
            Ok(output) => push_event(
                &queue,
                if active {
                    GuiEvent::AreaActivateUnavailable {
                        message: String::from_utf8_lossy(&output.stderr).to_string(),
                    }
                } else {
                    GuiEvent::AreaDeactivateUnavailable {
                        message: String::from_utf8_lossy(&output.stderr).to_string(),
                    }
                },
            ),
            Err(error) => push_event(
                &queue,
                if active {
                    GuiEvent::AreaActivateUnavailable {
                        message: error.to_string(),
                    }
                } else {
                    GuiEvent::AreaDeactivateUnavailable {
                        message: error.to_string(),
                    }
                },
            ),
        }
    });
}

fn ui_can_load_areas(ui: &Ui) -> bool {
    !ui.bridge.text().trim().is_empty() && !ui.app_key.text().trim().is_empty()
}

fn ui_event_queue(ui: &Ui) -> Arc<Mutex<VecDeque<GuiEvent>>> {
    ui.event_queue.clone()
}

fn selected_area_index(areas: &[AreaOption], previous_area: &str) -> Option<usize> {
    if previous_area.is_empty() {
        return (!areas.is_empty()).then_some(0);
    }
    areas
        .iter()
        .position(|area| area.id == previous_area || area.name == previous_area)
        .or_else(|| (!areas.is_empty()).then_some(0))
}

fn current_area_ref(ui: &Ui) -> String {
    let selected = ui.area_select.selected() as usize;
    ui.area_options
        .borrow()
        .get(selected)
        .map(|area| area.id.clone())
        .unwrap_or_else(|| ui.area.text().trim().to_string())
}

fn start_sync(ui: &Ui, state: &Rc<RefCell<AppState>>) -> anyhow::Result<()> {
    start_sync_with_log_reset(ui, state, true)
}

fn start_sync_with_log_reset(
    ui: &Ui,
    state: &Rc<RefCell<AppState>>,
    clear_logs: bool,
) -> anyhow::Result<()> {
    let bridge = ui.bridge.text().trim().to_string();
    let area = current_area_ref(ui);
    ui.area.set_text(&area);
    let app_key = ui.app_key.text().to_string();
    let client_key = ui.client_key.text().to_string();
    if !ui.area_enabled.is_active() {
        anyhow::bail!("zone is off");
    }

    if bridge.is_empty() || area.is_empty() || app_key.is_empty() || client_key.is_empty() {
        anyhow::bail!("bridge address, zone, and keys are required");
    }

    let duration_ms = ui.duration.value_as_int();
    let brightness = percent_to_fraction(ui.intensity.value());
    let smoothing = percent_to_fraction(ui.reactivity.value());
    let profile = ui.profile.text().trim().to_string();
    let sync_mode = screen_sync_mode(ui.sync_mode.get());
    let preset = preset_for_sync_mode(sync_mode);
    let color_profile = color_profile_for_sync_mode(sync_mode);
    write_current_env_file(ui)?;
    if clear_logs {
        ui.logs.set_text("");
    }
    append_log(&ui.logs, "Starting sync…\n");
    append_log(
        &ui.logs,
        &format!("Mode {} ({preset}).\n", sync_mode.as_env_value()),
    );
    append_log(
        &ui.logs,
        &format!("Duration {} ms (0 = until Stop).\n", duration_ms),
    );

    let mut command = Command::new(resolve_lumaway_binary());
    command
        .arg("sync")
        .arg("--sync-mode")
        .arg(sync_mode.as_env_value())
        .arg("--preset")
        .arg(preset)
        .arg("--duration-ms")
        .arg(duration_ms.to_string())
        .arg("--brightness")
        .arg(format_fraction(brightness))
        .arg("--smoothing")
        .arg(format_fraction(smoothing))
        .arg("--color-profile")
        .arg(color_profile)
        .env("LUMAWAY_BRIDGE", bridge)
        .env("LUMAWAY_AREA", area)
        .env("LUMAWAY_APP_KEY", app_key)
        .env("LUMAWAY_CLIENT_KEY", client_key)
        .env("RUST_LOG", "lumaway=info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !profile.is_empty() {
        command.env("LUMAWAY_PROFILE", profile);
    }
    if let Ok(saved) = read_env_file() {
        for key in [
            "LUMAWAY_BRIDGE_ID",
            "LUMAWAY_HUE_PIN_CERTS",
            "LUMAWAY_HUE_PIN_MODE",
            "LUMAWAY_DTLS_IDENTITY",
            "LUMAWAY_DTLS_USE_APP_KEY",
            "LUMAWAY_DTLS_USE_APPLICATION_ID",
        ] {
            if let Some(value) = saved.get(key) {
                if !value.trim().is_empty() {
                    command.env(key, value.trim());
                }
            }
        }
    }

    let mut child = command.spawn()?;
    if let Some(stdout) = child.stdout.take() {
        spawn_reader(stdout, state.borrow().log_queue.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_reader(stderr, state.borrow().log_queue.clone());
    }

    {
        let mut state = state.borrow_mut();
        state.pending_restart = false;
        state.child = Some(child);
    }
    set_running_state(ui, true);
    Ok(())
}

fn write_current_env_file(ui: &Ui) -> anyhow::Result<()> {
    let area = current_area_ref(ui);
    ui.area.set_text(&area);
    write_env_file(env_file_values_from_ui(ui, &area))
}

fn sync_running(state: &Rc<RefCell<AppState>>) -> bool {
    state.borrow().child.is_some()
}

fn schedule_sync_restart(ui: &Ui, state: &Rc<RefCell<AppState>>) {
    let should_stop = {
        let mut state = state.borrow_mut();
        if state.child.is_some() {
            let should_stop = !state.pending_restart;
            state.pending_restart = true;
            should_stop
        } else {
            false
        }
    };
    if should_stop {
        append_log(&ui.logs, "Applying new brightness…\n");
        stop_sync(ui, state);
    }
}

fn stop_sync(ui: &Ui, state: &Rc<RefCell<AppState>>) {
    let mut state = state.borrow_mut();
    if let Some(child) = state.child.as_mut() {
        let _ = Command::new("kill")
            .arg("-INT")
            .arg(child.id().to_string())
            .status();
        append_log(&ui.logs, "Stopping sync…\n");
    }
}

fn start_log_pump(ui: &Ui, state: Rc<RefCell<AppState>>) {
    let ui = ui.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        let mut finished = false;
        let (events, auto_quit_after_sync, sync_running, restart_requested) = {
            let mut state = state.borrow_mut();
            let auto_quit_after_sync = state.auto_quit_after_sync;
            if let Some(child) = state.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        finished = true;
                    }
                    Ok(None) => {}
                    Err(_error) => {
                        finished = true;
                    }
                }
            }

            let lines = drain_logs(&state.log_queue);
            for line in lines {
                if state.echo_logs {
                    println!("{line}");
                }
                append_log(&ui.logs, &line);
                append_log(&ui.logs, "\n");
            }

            if finished {
                state.child = None;
            }
            let restart_requested = finished && state.pending_restart;
            (
                drain_events(&state.event_queue),
                auto_quit_after_sync,
                state.child.is_some(),
                restart_requested,
            )
        };

        for event in events {
            handle_event(&ui, event, sync_running);
        }

        if finished {
            set_running_state(&ui, false);
            if restart_requested && ui.area_enabled.is_active() {
                if let Err(error) = start_sync_with_log_reset(&ui, &state, false) {
                    append_log(&ui.logs, &format!("error: {error}\n"));
                    state.borrow_mut().pending_restart = false;
                }
            } else if auto_quit_after_sync {
                ui.window.close();
            }
        }

        glib::ControlFlow::Continue
    });
}

fn handle_event(ui: &Ui, event: GuiEvent, sync_running: bool) {
    match event {
        GuiEvent::BridgesDiscovered(bridges) => match bridges.as_slice() {
            [] => {
                set_connection_header(ui, "Bridge not found", "Open Settings", "warning");
                append_log(&ui.logs, "No bridge found automatically.\n");
            }
            [bridge] => {
                ui.bridge.set_text(&bridge.ip);
                if !bridge.id.is_empty() {
                    *ui.bridge_id.borrow_mut() = bridge.id.clone();
                    refresh_bridge_id_displays(ui);
                    let _ = persist_bridge_id_env(&bridge.id);
                }
                set_connection_header(ui, "Bridge found", "Checking…", "warning");
                append_log(
                    &ui.logs,
                    &format!("Bridge detected automatically: {}\n", bridge.ip),
                );
                if ui_can_load_areas(ui) {
                    load_areas(ui);
                }
            }
            bridges => {
                let first = &bridges[0];
                ui.bridge.set_text(&first.ip);
                if !first.id.is_empty() {
                    *ui.bridge_id.borrow_mut() = first.id.clone();
                    refresh_bridge_id_displays(ui);
                    let _ = persist_bridge_id_env(&first.id);
                }
                set_connection_header(ui, "Bridge found", "Checking…", "warning");
                append_log(
                    &ui.logs,
                    &format!(
                        "{} bridges detected. First selected: {}\n",
                        bridges.len(),
                        first.ip
                    ),
                );
                if ui_can_load_areas(ui) {
                    load_areas(ui);
                }
            }
        },
        GuiEvent::AuthCreated {
            app_key,
            client_key,
        } => {
            ui.app_key.set_text(&app_key);
            ui.client_key.set_text(&client_key);
            update_health(ui);
            match write_current_env_file(ui) {
                Ok(()) => append_log(
                    &ui.logs,
                    "Pairing succeeded and keys were saved. Loading zones…\n",
                ),
                Err(error) => append_log(
                    &ui.logs,
                    &format!("Pairing succeeded, but settings could not be saved: {error}\n"),
                ),
            }
            load_areas(ui);
        }
        GuiEvent::BridgeInfoLoaded { id, name } => {
            if !id.is_empty() {
                *ui.bridge_id.borrow_mut() = id.clone();
                refresh_bridge_id_displays(ui);
                match persist_bridge_id_env(&id) {
                    Ok(()) => append_log(
                        &ui.logs,
                        &format!("Bridge id saved (LUMAWAY_BRIDGE_ID={id})\n"),
                    ),
                    Err(error) => append_log(
                        &ui.logs,
                        &format!("warning: could not save bridge id: {error}\n"),
                    ),
                }
            }
            set_connection_header(
                ui,
                BRIDGE_STATUS_CONNECTED,
                &format_bridge_title(&name, ui.bridge_id.borrow().as_str()),
                "connected",
            );
            append_log(&ui.logs, &format!("Bridge name: {name}\n"));
        }
        GuiEvent::BridgeInfoUnavailable { message } => {
            set_connection_header(ui, "Bridge not connected", "Check Settings", "warning");
            append_log(
                &ui.logs,
                &format!("Could not read bridge name: {}\n", message.trim()),
            );
        }
        GuiEvent::AreasLoaded(areas) => {
            let previous_area = ui.area.text().trim().to_string();
            let labels: Vec<String> = areas.iter().map(|area| area.name.clone()).collect();
            let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
            ui.area_model
                .splice(0, ui.area_model.n_items(), &label_refs);
            *ui.area_options.borrow_mut() = areas.clone();
            if let Some(index) = selected_area_index(&areas, &previous_area) {
                ui.suppress_area_toggle.set(true);
                ui.area_select.set_selected(index as u32);
                ui.area.set_text(&areas[index].id);
                ui.area_enabled.set_active(!areas[index].id.is_empty());
                ui.suppress_area_toggle.set(false);
            }
            if ui.connection_status.text().as_str() != BRIDGE_STATUS_CONNECTED {
                set_connection_header(ui, BRIDGE_STATUS_CONNECTED, "Bridge connected", "connected");
            }
            update_zone_controls(ui, sync_running);
            update_area_lights_subtitle(ui);
            append_log(&ui.logs, &format!("{} zones loaded.\n", areas.len()));
        }
        GuiEvent::AreaActivated { name, lights } => {
            append_log(
                &ui.logs,
                &format!(
                    "Zone on: {name} ({lights} light{}). Press Start to run sync.\n",
                    if lights != 1 { "s" } else { "" }
                ),
            );
        }
        GuiEvent::AreaActivateUnavailable { message } => {
            append_log(
                &ui.logs,
                &format!("error: could not activate zone: {}\n", message.trim()),
            );
        }
        GuiEvent::AreaDeactivated { name, lights } => {
            append_log(
                &ui.logs,
                &format!(
                    "Zone off: {name} ({lights} light{}).\n",
                    if lights != 1 { "s" } else { "" }
                ),
            );
        }
        GuiEvent::AreaDeactivateUnavailable { message } => {
            append_log(
                &ui.logs,
                &format!("error: could not deactivate zone: {}\n", message.trim()),
            );
        }
        GuiEvent::ProfilesListed(profiles) => {
            if profiles.is_empty() {
                append_log(&ui.logs, "No capture profiles found.\n");
            } else {
                append_log(&ui.logs, "Capture profiles:\n");
                for profile in profiles {
                    append_log(&ui.logs, &format!("- {profile}\n"));
                }
            }
        }
        GuiEvent::ProfileCalibrated { profile, output } => {
            ui.profile.set_text(&profile);
            let _ = write_current_env_file(ui);
            append_log(
                &ui.logs,
                &format!("Capture profile `{profile}` calibrated and saved.\n"),
            );
            for line in output.lines().filter(|line| !line.trim().is_empty()) {
                append_log(&ui.logs, line);
                append_log(&ui.logs, "\n");
            }
        }
        GuiEvent::CaptureQualityMeasured(output) => {
            if output.trim().is_empty() {
                append_log(&ui.logs, "Capture quality completed without output.\n");
            } else if let Some(summary) = format_capture_quality_summary(&output) {
                append_log(&ui.logs, &summary);
            } else {
                append_log(&ui.logs, "Capture quality:\n");
                append_log(&ui.logs, output.trim());
                append_log(&ui.logs, "\n");
            }
        }
        GuiEvent::Error(error) => {
            let error = error.trim();
            if is_hue_authentication_error(error) {
                set_connection_header(ui, "Pairing required", "Press Pair in Settings", "warning");
                append_log(
                    &ui.logs,
                    "error: saved Hue application key was rejected. Press the bridge button, then press Pair in Settings.\n",
                );
            } else if ui_can_load_areas(ui) {
                set_connection_header(ui, "Bridge not connected", "Check Settings", "warning");
                append_log(&ui.logs, &format!("error: {error}\n"));
            } else {
                update_health(ui);
                append_log(&ui.logs, &format!("error: {error}\n"));
            }
        }
    }
}

fn apply_sync_idle_button_style(ui: &Ui) {
    ui.start.set_label("Start sync");
    ui.start.remove_css_class("destructive-action");
    ui.start.add_css_class("suggested-action");
    ui.start.add_css_class("primary-sync");
}

fn apply_sync_running_button_style(ui: &Ui) {
    ui.start.set_label("Stop syncing");
    ui.start.remove_css_class("suggested-action");
    ui.start.remove_css_class("primary-sync");
    ui.start.add_css_class("destructive-action");
}

fn set_running_state(ui: &Ui, running: bool) {
    if running {
        apply_sync_running_button_style(ui);
    } else {
        apply_sync_idle_button_style(ui);
    }
    ui.auth.set_sensitive(!running);
    ui.discover.set_sensitive(!running);
    ui.bridge.set_sensitive(!running);
    ui.area.set_sensitive(!running);
    ui.area_select.set_sensitive(!running);
    ui.area_enabled
        .set_sensitive(!current_area_ref(ui).trim().is_empty());
    ui.app_key.set_sensitive(!running);
    ui.client_key.set_sensitive(!running);
    ui.duration.set_sensitive(!running);
    ui.reactivity.set_sensitive(!running);
    ui.autostart.set_sensitive(!running);
    update_zone_controls(ui, running);
}

fn update_zone_controls(ui: &Ui, running: bool) {
    let has_zone = !current_area_ref(ui).trim().is_empty();
    let zone_ready = has_zone && ui.area_enabled.is_active();
    ui.intensity.set_sensitive(has_zone);
    ui.start.set_sensitive(running || zone_ready);
}

fn update_area_lights_subtitle(ui: &Ui) {
    let selected = ui.area_select.selected() as usize;
    let caption = ui
        .area_options
        .borrow()
        .get(selected)
        .and_then(|opt| opt.lights)
        .map(|n| match n {
            0 => "No lights in this zone".to_string(),
            1 => "1 light syncing".to_string(),
            n => format!("{n} lights syncing"),
        })
        .unwrap_or_default();
    ui.area_lights_subtitle.set_text(&caption);
    ui.area_lights_subtitle.set_visible(!caption.is_empty());
}

fn update_health(ui: &Ui) {
    if ui.bridge.text().trim().is_empty() {
        set_connection_header(ui, "Bridge not configured", "Open Settings", "warning");
    } else if ui.app_key.text().trim().is_empty() {
        set_connection_header(ui, "Bridge found", "Pairing required", "warning");
    } else {
        set_connection_header(ui, "Bridge configured", "Checking…", "warning");
    }
}

fn format_bridge_title(name: &str, bridge_id: &str) -> String {
    let name = name.trim();
    let bridge_id = bridge_id.trim();
    if name.is_empty() {
        return bridge_id.to_string();
    }
    if bridge_id.is_empty() {
        return name.to_string();
    }
    format!("{name} ({bridge_id})")
}

fn set_connection_header(ui: &Ui, state: &str, title: &str, tone: &str) {
    ui.connection_status.set_text(state);
    ui.bridge_display.set_text(title);
    for class in ["connected", "warning"] {
        ui.connection_status.remove_css_class(class);
    }
    ui.connection_status.add_css_class(tone);
}

fn spawn_reader<T: std::io::Read + Send + 'static>(reader: T, queue: Arc<Mutex<VecDeque<String>>>) {
    thread::spawn(move || {
        let reader = BufReader::new(reader);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(mut queue) = queue.lock() {
                queue.push_back(line);
                while queue.len() > 500 {
                    queue.pop_front();
                }
            }
        }
    });
}

fn drain_logs(queue: &Arc<Mutex<VecDeque<String>>>) -> Vec<String> {
    let Ok(mut queue) = queue.lock() else {
        return Vec::new();
    };
    queue.drain(..).collect()
}

fn push_event(queue: &Arc<Mutex<VecDeque<GuiEvent>>>, event: GuiEvent) {
    if let Ok(mut queue) = queue.lock() {
        queue.push_back(event);
    }
}

fn drain_events(queue: &Arc<Mutex<VecDeque<GuiEvent>>>) -> Vec<GuiEvent> {
    let Ok(mut queue) = queue.lock() else {
        return Vec::new();
    };
    queue.drain(..).collect()
}

fn parse_auth_output(bytes: &[u8]) -> Result<(String, String), String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let app_key = value
        .get("app_key")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "auth response did not contain app_key".to_string())?
        .to_string();
    let client_key = value
        .get("client_key")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "auth response did not contain client_key".to_string())?
        .to_string();
    Ok((app_key, client_key))
}

fn parse_areas_output(bytes: &[u8]) -> Result<Vec<AreaOption>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let areas = value
        .as_array()
        .ok_or_else(|| "areas response was not a JSON array".to_string())?;
    Ok(areas
        .iter()
        .filter_map(|area| {
            let id = area.get("id")?.as_str()?.to_string();
            let name = area.get("name")?.as_str()?.to_string();
            let lights = area
                .get("lights")
                .and_then(serde_json::Value::as_u64)
                .map(|n| n as usize);
            Some(AreaOption { id, name, lights })
        })
        .collect())
}

fn parse_bridge_discovery_output(bytes: &[u8]) -> Result<Vec<BridgeOption>, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let bridges = value
        .as_array()
        .ok_or_else(|| "discovery response was not a JSON array".to_string())?;
    Ok(bridges
        .iter()
        .filter_map(|bridge| {
            let ip = bridge
                .get("internalipaddress")
                .or_else(|| bridge.get("ip"))
                .and_then(serde_json::Value::as_str)?
                .trim()
                .to_string();
            if ip.is_empty() {
                return None;
            }
            let id = bridge
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            Some(BridgeOption { ip, id })
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BridgeInfoParsed {
    id: String,
    name: String,
}

fn parse_bridge_info_output(bytes: &[u8]) -> Result<BridgeInfoParsed, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "bridge info response did not contain a name".to_string())?;
    Ok(BridgeInfoParsed { id, name })
}

fn persist_bridge_id_env(bridge_id: &str) -> anyhow::Result<()> {
    let bridge_id = bridge_id.trim();
    if bridge_id.is_empty() {
        return Ok(());
    }
    lumaway_core::upsert_env_file(
        &lumaway_core::lumaway_main_env_path(),
        &[("LUMAWAY_BRIDGE_ID", bridge_id)],
    )
    .map_err(|error| anyhow::anyhow!("{error}"))
}

fn parse_area_state_output(bytes: &[u8]) -> Result<(String, usize), String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let name = value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "area response did not contain a name".to_string())?
        .to_string();
    let lights = value
        .get("lights")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "area response did not contain a lights count".to_string())?
        as usize;
    Ok((name, lights))
}

fn parse_profile_list_output(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            line.split_whitespace()
                .find_map(|part| part.strip_prefix("name="))
                .map(str::to_string)
        })
        .collect()
}

fn format_capture_quality_summary(output: &str) -> Option<String> {
    let line = output
        .lines()
        .find(|line| line.trim_start().starts_with("capture_quality "))?;
    let fields = parse_key_value_words(line);
    let recommendation = fields.get("recommendation")?;
    let area = fields.get("area_name").map(String::as_str).unwrap_or("?");
    let frames = fields.get("frames").map(String::as_str).unwrap_or("?");
    let channels = fields.get("channels").map(String::as_str).unwrap_or("?");
    let backend = fields
        .get("capture_backend")
        .map(String::as_str)
        .unwrap_or("?");
    let luma = fields.get("avg_luma").map(String::as_str).unwrap_or("?");
    let saturation = fields
        .get("avg_saturation")
        .map(String::as_str)
        .unwrap_or("?");
    let frame_delta = fields
        .get("avg_frame_delta")
        .map(String::as_str)
        .unwrap_or("?");
    let separation = fields
        .get("avg_channel_separation")
        .map(String::as_str)
        .unwrap_or("?");
    let dark_frames = fields.get("dark_frames").map(String::as_str).unwrap_or("?");
    let warnings = fields.get("warnings").map(String::as_str).unwrap_or("none");
    let action = fields
        .get("hint")
        .map(|hint| capture_quality_hint_label(hint))
        .unwrap_or_else(|| capture_quality_recommendation_label(recommendation));

    Some(format!(
        "Capture quality: {recommendation}\n- area: {area}, action: {action}\n- backend: {backend}, frames: {frames}, channels: {channels}\n- luma: {luma}, saturation: {saturation}\n- frame delta: {frame_delta}, channel separation: {separation}, dark frames: {dark_frames}\n- warnings: {warnings}\n"
    ))
}

fn capture_quality_hint_label(hint: &str) -> &'static str {
    match hint {
        "choose_multi_channel_area_for_correlation_test" => {
            "choose TV or another multi-light entertainment area to test light correlation"
        }
        "rerun_backend_probe_or_raise_brightness" => {
            "run backend probe again, then raise brightness only if capture is not dark"
        }
        "test_with_moving_or_contrasting_windows" => {
            "move contrasting windows while measuring; static content cannot prove responsiveness"
        }
        "use_region_sampling_or_adjust_channel_regions" => {
            "use region sampling and adjust channel regions/profile mapping"
        }
        "try_boosted_color_profile" => "try the boosted color profile, then rerun Quality",
        "try_vivid_or_game_color_profile" => {
            "try the vivid, game, or boosted color profile, then rerun Quality"
        }
        "capture_is_usable_tune_color_profile_if_needed" => {
            "capture is usable; tune color profile if the visual result is still weak"
        }
        _ => capture_quality_recommendation_label(hint),
    }
}

fn capture_quality_recommendation_label(recommendation: &str) -> &'static str {
    match recommendation {
        "single_channel_area" => "choose a multi-light entertainment area",
        "capture_too_dark" => "rerun backend probe or check Portal/backend brightness",
        "low_temporal_variation" => "test with changing window colors",
        "low_spatial_separation" => "adjust sampling regions or Hue channel mapping",
        "low_saturation" => "try the boosted color profile",
        "usable" => "capture looks usable",
        _ => "inspect sample-debug output",
    }
}

fn parse_key_value_words(line: &str) -> HashMap<String, String> {
    line.split_whitespace()
        .filter_map(|word| {
            let (key, value) = word.split_once('=')?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn is_hue_authentication_error(error: &str) -> bool {
    error.contains("Hue bridge authentication failed")
        || error.contains("saved Hue application key was rejected")
}

fn append_log(buffer: &gtk::TextBuffer, text: &str) {
    let mut iter = buffer.end_iter();
    buffer.insert(&mut iter, text);
}

fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("lumaway")
        .join("lumaway.env")
}

fn read_env_file() -> anyhow::Result<HashMap<String, String>> {
    let path = config_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(path)?;
    let mut values = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(values)
}

struct EnvFileValues {
    sync_mode: SyncMode,
    bridge: String,
    bridge_id: String,
    area: String,
    app_key: String,
    client_key: String,
    duration_ms: i32,
    brightness: f64,
    smoothing: f64,
    profile: String,
    color_profile: String,
    autostart: bool,
}

fn env_file_values_from_ui(ui: &Ui, area: &str) -> EnvFileValues {
    EnvFileValues {
        sync_mode: screen_sync_mode(ui.sync_mode.get()),
        bridge: ui.bridge.text().trim().to_string(),
        bridge_id: ui.bridge_id.borrow().trim().to_string(),
        area: area.trim().to_string(),
        app_key: ui.app_key.text().to_string(),
        client_key: ui.client_key.text().to_string(),
        duration_ms: ui.duration.value_as_int(),
        brightness: percent_to_fraction(ui.intensity.value()),
        smoothing: percent_to_fraction(ui.reactivity.value()),
        profile: ui.profile.text().trim().to_string(),
        color_profile: color_profile_for_sync_mode(ui.sync_mode.get()).to_string(),
        autostart: ui.autostart.is_active(),
    }
}

fn write_env_file(values: EnvFileValues) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let profile = sanitize_profile_name(&values.profile);
    let bridge_id_line = if values.bridge_id.trim().is_empty() {
        String::new()
    } else {
        format!("LUMAWAY_BRIDGE_ID={}\n", values.bridge_id.trim())
    };
    let text = format!(
        "{config_version_key}={config_version}\n{sync_mode_key}={sync_mode}\nLUMAWAY_BRIDGE={bridge}\n{bridge_id_line}LUMAWAY_AREA={area}\nLUMAWAY_APP_KEY={app_key}\nLUMAWAY_CLIENT_KEY={client_key}\nLUMAWAY_PROFILE={profile}\nLUMAWAY_DURATION_MS={duration_ms}\nLUMAWAY_BRIGHTNESS={brightness}\nLUMAWAY_REACTIVITY={reactivity}\nLUMAWAY_COLOR_PROFILE={color_profile}\nLUMAWAY_AUTOSTART_SYNC={autostart}\nRUST_LOG=lumaway=info\n",
        config_version_key = CONFIG_VERSION_KEY,
        config_version = CURRENT_CONFIG_VERSION,
        sync_mode_key = SYNC_MODE_KEY,
        sync_mode = values.sync_mode.as_env_value(),
        bridge = values.bridge,
        area = values.area,
        app_key = values.app_key,
        client_key = values.client_key,
        duration_ms = values.duration_ms,
        brightness = format_fraction(values.brightness),
        reactivity = format_fraction(values.smoothing),
        color_profile = sanitize_color_profile(&values.color_profile),
        autostart = if values.autostart { "true" } else { "false" },
    );
    fs::write(&path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn sanitize_profile_name(value: &str) -> String {
    let value = value.trim();
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
    {
        String::new()
    } else {
        value.to_string()
    }
}

/// When `LUMAWAY_BIN` is set, it must be an absolute path to a regular file that is not
/// world-writable (reduces risk of a tampered binary on shared systems). Invalid values are logged
/// and ignored so the GUI still starts with the default resolution order.
fn lumaway_bin_override_rejection_reason(path: &Path) -> Option<&'static str> {
    if !path.is_absolute() {
        return Some("LUMAWAY_BIN must be an absolute path");
    }
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return Some("LUMAWAY_BIN path is not readable"),
    };
    if !meta.is_file() {
        return Some("LUMAWAY_BIN must be a regular file");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o002 != 0 {
            return Some("LUMAWAY_BIN must not be world-writable");
        }
    }
    None
}

fn resolve_lumaway_binary() -> PathBuf {
    if let Some(raw) = std::env::var_os("LUMAWAY_BIN") {
        let path = PathBuf::from(raw);
        if let Some(reason) = lumaway_bin_override_rejection_reason(&path) {
            eprintln!(
                "lumaway-gui: ignoring LUMAWAY_BIN ({reason}); path={}",
                path.display()
            );
        } else {
            return path;
        }
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let sibling = dir.join("lumaway");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    home_dir().join(".local/bin/lumaway")
}

fn initial_sync_mode(saved: &HashMap<String, String>) -> SyncMode {
    screen_sync_mode(resolve_sync_mode(
        saved.get(SYNC_MODE_KEY).map(String::as_str),
        saved.get(LEGACY_PRESET_KEY).map(String::as_str),
    ))
}

fn screen_sync_mode(mode: SyncMode) -> SyncMode {
    match mode {
        SyncMode::Music => DEFAULT_SYNC_MODE,
        mode => mode,
    }
}

fn preset_for_sync_mode(mode: SyncMode) -> &'static str {
    screen_sync_mode(mode)
        .default_preset()
        .unwrap_or("video-wayland")
}

fn color_profile_for_sync_mode(mode: SyncMode) -> &'static str {
    match screen_sync_mode(mode) {
        SyncMode::Video => "vivid",
        SyncMode::Game => "game",
        SyncMode::Desktop => "desktop",
        SyncMode::Music => "vivid",
    }
}

fn initial_duration_ms(saved: &HashMap<String, String>) -> i32 {
    std::env::var("LUMAWAY_GUI_DURATION_MS")
        .ok()
        .or_else(|| saved.get("LUMAWAY_DURATION_MS").cloned())
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value >= 0)
        .unwrap_or(0)
}

fn initial_percent(saved: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    saved
        .get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value * 100.0)
        .filter(|value| (0.0..=100.0).contains(value))
        .unwrap_or(default)
}

fn saved_flag(saved: &HashMap<String, String>, key: &str) -> bool {
    saved
        .get(key)
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn percent_to_fraction(value: f64) -> f64 {
    (value / 100.0).clamp(0.0, 1.0)
}

fn format_fraction(value: f64) -> String {
    format!("{:.2}", value.clamp(0.0, 1.0))
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::{
        bridge_id_display_text, color_profile_for_sync_mode, format_bridge_title,
        format_capture_quality_summary, initial_sync_mode, is_hue_authentication_error,
        lumaway_bin_override_rejection_reason, parse_area_state_output, parse_areas_output,
        parse_auth_output, parse_bridge_discovery_output, parse_bridge_info_output,
        parse_profile_list_output, preset_for_sync_mode, sanitize_color_profile,
        sanitize_profile_name, selected_area_index,
    };
    use lumaway_core::SyncMode;
    use std::collections::HashMap;
    use std::path::Path;

    fn saved(values: &[(&str, &str)]) -> HashMap<String, String> {
        values
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    fn capture_status_from_log(line: &str) -> Option<String> {
        if line.contains("selected portal stream") {
            return Some("Capture: display selected".to_string());
        }
        if line.contains("effective_capture_backend=Gl")
            || line.contains("capture_backend=gl")
            || line.contains("capture_backend=Gl")
        {
            return Some("Capture: GL".to_string());
        }
        if line.contains("effective_capture_backend=Cpu")
            || line.contains("capture_backend=cpu")
            || line.contains("capture_backend=Cpu")
        {
            return Some("Capture: CPU".to_string());
        }
        None
    }

    #[test]
    fn parses_auth_output() {
        let (app_key, client_key) =
            parse_auth_output(br#"{"app_key":"app","client_key":"client"}"#).unwrap();

        assert_eq!(app_key, "app");
        assert_eq!(client_key, "client");
    }

    #[test]
    fn rejects_auth_output_without_client_key() {
        let error = parse_auth_output(br#"{"app_key":"app"}"#).unwrap_err();

        assert!(error.contains("client_key"));
    }

    #[test]
    fn parses_area_options() {
        let areas = parse_areas_output(
            br#"[
                {"id":"01234567-89ab-cdef-0123-456789abcdef","name":"TV","channels":[],"lights":5},
                {"id":"fedcba98-7654-3210-fedc-ba9876543210","name":"Desk","channels":[]}
            ]"#,
        )
        .unwrap();

        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].id, "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(areas[0].name, "TV");
        assert_eq!(areas[0].lights, Some(5));
        assert_eq!(areas[1].name, "Desk");
        assert_eq!(areas[1].lights, None);
    }

    #[test]
    fn parses_bridge_discovery_output() {
        let bridges = parse_bridge_discovery_output(
            br#"[
                {"id":"001788fffe123456","internalipaddress":"192.168.1.108"}
            ]"#,
        )
        .unwrap();

        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].ip, "192.168.1.108");
        assert_eq!(bridges[0].id, "001788fffe123456");
    }

    #[test]
    fn parses_bridge_info_output() {
        let info = parse_bridge_info_output(br#"{"id":"bridge-1","name":"Salon Bridge"}"#).unwrap();

        assert_eq!(info.id, "bridge-1");
        assert_eq!(info.name, "Salon Bridge");
    }

    #[test]
    fn bridge_id_display_text_placeholder_when_empty() {
        assert_eq!(bridge_id_display_text(""), "— (load zones after pairing)");
        assert_eq!(bridge_id_display_text("abc"), "abc");
    }

    #[test]
    fn format_bridge_title_includes_id_when_present() {
        assert_eq!(
            format_bridge_title("Salon", "001788fffe123456"),
            "Salon (001788fffe123456)"
        );
        assert_eq!(format_bridge_title("Salon", ""), "Salon");
    }

    #[test]
    fn parses_profile_list_output() {
        let profiles = parse_profile_list_output(
            "profile name=alpha path=/tmp/lumaway/profiles/alpha.env\n\
             ignored line\n\
             profile name=zeta path=/tmp/lumaway/profiles/zeta.env\n",
        );

        assert_eq!(profiles, vec!["alpha", "zeta"]);
    }

    #[test]
    fn formats_capture_quality_summary() {
        let summary = format_capture_quality_summary(
            "capture_quality area_id=area area_name=TV capture_backend=cpu sampling=Region frames=30 channels=2 avg_luma=84.2 avg_saturation=0.318 avg_frame_delta=12.4 max_frame_delta=60.0 avg_channel_separation=31.7 dark_frames=0 recommendation=usable hint=capture_is_usable_tune_color_profile_if_needed warnings=none",
        )
        .unwrap();

        assert!(summary.contains("Capture quality: usable"));
        assert!(summary.contains("area: TV, action: capture is usable"));
        assert!(summary.contains("backend: cpu"));
        assert!(summary.contains("luma: 84.2"));
        assert!(summary.contains("channel separation: 31.7"));
        assert!(summary.contains("warnings: none"));
    }

    #[test]
    fn formats_single_channel_capture_quality_action() {
        let summary = format_capture_quality_summary(
            "capture_quality area_id=area area_name=Bureau capture_backend=cpu sampling=Region frames=30 channels=1 avg_luma=66.1 avg_saturation=0.540 avg_frame_delta=0.0 max_frame_delta=0.0 avg_channel_separation=0.0 dark_frames=0 recommendation=single_channel_area hint=choose_multi_channel_area_for_correlation_test warnings=low_temporal_variation",
        )
        .unwrap();

        assert!(summary.contains("Capture quality: single_channel_area"));
        assert!(summary.contains("choose TV or another multi-light"));
        assert!(summary.contains("channels: 1"));
    }

    #[test]
    fn formats_capture_quality_secondary_warnings() {
        let summary = format_capture_quality_summary(
            "capture_quality area_id=area area_name=TV capture_backend=cpu sampling=Region frames=30 channels=2 avg_luma=39.1 avg_saturation=0.067 avg_frame_delta=0.0 max_frame_delta=0.0 avg_channel_separation=8.7 dark_frames=0 recommendation=low_temporal_variation hint=test_with_moving_or_contrasting_windows warnings=low_luma,low_saturation,low_temporal_variation",
        )
        .unwrap();

        assert!(summary.contains("Capture quality: low_temporal_variation"));
        assert!(summary.contains("warnings: low_luma,low_saturation,low_temporal_variation"));
        assert!(summary.contains("channel separation: 8.7"));
    }

    #[test]
    fn detects_hue_authentication_errors() {
        assert!(is_hue_authentication_error(
            "Error: Hue bridge authentication failed"
        ));
        assert!(is_hue_authentication_error(
            "saved Hue application key was rejected; pair again"
        ));
        assert!(!is_hue_authentication_error("portal request failed"));
    }

    #[test]
    fn parses_area_state_output() {
        let (name, lights) =
            parse_area_state_output(br#"{"active":true,"name":"TV","lights":2}"#).unwrap();

        assert_eq!(name, "TV");
        assert_eq!(lights, 2);
    }

    #[test]
    fn keeps_saved_area_after_automatic_area_loading() {
        let areas = parse_areas_output(
            br#"[
                {"id":"living-room","name":"Salon","channels":[]},
                {"id":"tv-zone","name":"TV","channels":[]}
            ]"#,
        )
        .unwrap();

        assert_eq!(selected_area_index(&areas, "tv-zone"), Some(1));
        assert_eq!(selected_area_index(&areas, "TV"), Some(1));
        assert_eq!(selected_area_index(&areas, ""), Some(0));
        assert_eq!(selected_area_index(&areas, "missing"), Some(0));
    }

    #[test]
    fn sanitizes_profile_names_before_writing_env() {
        assert_eq!(sanitize_profile_name(" tv "), "tv");
        assert_eq!(sanitize_color_profile("boosted"), "boosted");
        assert_eq!(sanitize_profile_name("../tv"), "");
        assert_eq!(sanitize_profile_name("room/tv"), "");
        assert_eq!(sanitize_profile_name(".."), "");
    }

    #[test]
    fn initial_sync_mode_reads_v1_config_and_legacy_preset() {
        assert_eq!(
            initial_sync_mode(&saved(&[("LUMAWAY_SYNC_MODE", "game")])),
            SyncMode::Game
        );
        assert_eq!(
            initial_sync_mode(&saved(&[("LUMAWAY_PRESET", "desktop-wayland")])),
            SyncMode::Desktop
        );
        assert_eq!(
            initial_sync_mode(&saved(&[("LUMAWAY_SYNC_MODE", "music")])),
            SyncMode::Video
        );
    }

    #[test]
    fn presets_and_color_profiles_follow_screen_modes() {
        assert_eq!(preset_for_sync_mode(SyncMode::Video), "video-wayland");
        assert_eq!(preset_for_sync_mode(SyncMode::Game), "game-wayland");
        assert_eq!(preset_for_sync_mode(SyncMode::Desktop), "desktop-wayland");
        assert_eq!(preset_for_sync_mode(SyncMode::Music), "video-wayland");

        assert_eq!(color_profile_for_sync_mode(SyncMode::Video), "vivid");
        assert_eq!(color_profile_for_sync_mode(SyncMode::Game), "game");
        assert_eq!(color_profile_for_sync_mode(SyncMode::Desktop), "desktop");
        assert_eq!(color_profile_for_sync_mode(SyncMode::Music), "vivid");
    }

    #[test]
    fn derives_capture_status_from_engine_logs() {
        assert_eq!(
            capture_status_from_log(
                "INFO lumaway: selected portal stream node_id=112 size=Some((2560, 1440))"
            ),
            Some("Capture: display selected".to_string())
        );
        assert_eq!(
            capture_status_from_log(
                "INFO lumaway: capture backend selected capture_backend=Auto effective_capture_backend=Gl"
            ),
            Some("Capture: GL".to_string())
        );
        assert_eq!(
            capture_status_from_log("sync_stats capture_backend=cpu interrupted=false frames=125"),
            Some("Capture: CPU".to_string())
        );
        assert_eq!(capture_status_from_log("unrelated log line"), None);
    }

    #[test]
    fn lumaway_bin_rejects_relative_paths() {
        assert!(lumaway_bin_override_rejection_reason(Path::new("lumaway")).is_some());
        assert!(lumaway_bin_override_rejection_reason(Path::new("./lumaway")).is_some());
    }

    #[test]
    fn lumaway_bin_rejects_nonexistent_absolute() {
        assert!(
            lumaway_bin_override_rejection_reason(Path::new("/no/such/lumaway-bin-xyz-999"))
                .is_some()
        );
    }

    #[test]
    fn lumaway_bin_accepts_typical_system_binary() {
        if Path::new("/bin/true").exists() {
            assert!(lumaway_bin_override_rejection_reason(Path::new("/bin/true")).is_none());
        }
    }

    #[cfg(unix)]
    #[test]
    fn lumaway_bin_rejects_world_writable_file() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join("lumaway_bin_override_test_world_writable");
        let _ = fs::remove_file(&path);
        fs::write(&path, b"#").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(lumaway_bin_override_rejection_reason(&path).is_some());
        let _ = fs::remove_file(&path);
    }
}
