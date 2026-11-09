//! Os botões do modo organizar pela janela (spec EG, seção 3; spec BO): as ferramentas e o zoom
//! rodam os comandos do registro, e a barra única de cima mostra o que cabe na largura e o que vale
//! com a seleção. A paleta flutuante e a destacada saíram com a spec BO.

use std::cell::RefCell;
use std::rc::Rc;

use lamina_editor::Command;
use lamina_editor::editor::PaletteTool;
use lamina_editor::organize_bar::{BAR_MENUS_ORDER, BarMenu, bar_fit, bar_groups};
use slint::{ComponentHandle, SharedString};

use crate::state::App;
use crate::{AppWindow, BarRow, OrganizeUi, PaletteUi};

type Shared = Rc<RefCell<App>>;

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<PaletteUi>();
    let a = app.clone();
    ui.on_command(move |name| {
        if let Some(command) = Command::from_name(&name) {
            crate::input::run_command(&a, command);
        }
    });
    let a = app.clone();
    ui.on_pick_tool(move |tool| {
        // A ferramenta mora no editor (spec ZR, regra 2).
        let mut app = a.borrow_mut();
        app.editor.set_tool(PaletteTool::from_index(tool));
        if let Some(w) = app.window.upgrade() {
            w.global::<OrganizeUi>().set_tool(tool);
        }
        refresh(&app);
    });
    let organize = window.global::<OrganizeUi>();
    let weak = window.as_weak();
    organize.on_bar_width(move |width| {
        // O que cabe na barra (spec BO, regra 3).
        let Some(w) = weak.upgrade() else {
            return;
        };
        let fit = bar_fit(f64::from(width));
        let ui = w.global::<OrganizeUi>();
        ui.set_bar_fit(fit.fit);
        ui.set_bar_menus(fit.menus);
        ui.set_bar_fields(fit.fields);
    });
    let a = app.clone();
    organize.on_zoom_slider(move |pos| {
        // O controle de zoom da lâmina (spec CD, regra 3): sem histórico.
        let mut app = a.borrow_mut();
        if app.editor.set_view_zoom_position(f64::from(pos)).is_some() {
            app.refresh_current();
        }
    });
}

/// O desenho de cada comando dos menus, do Lucide (spec DF, regra 7), embutido no programa.
fn icon_svg(command: Command) -> &'static [u8] {
    match command {
        Command::AlignLeft => include_bytes!("../icons/lucide/align-start-vertical.svg"),
        Command::AlignCenter => include_bytes!("../icons/lucide/align-center-vertical.svg"),
        Command::AlignRight => include_bytes!("../icons/lucide/align-end-vertical.svg"),
        Command::AlignTop => include_bytes!("../icons/lucide/align-start-horizontal.svg"),
        Command::AlignMiddle => include_bytes!("../icons/lucide/align-center-horizontal.svg"),
        Command::AlignBottom => include_bytes!("../icons/lucide/align-end-horizontal.svg"),
        Command::EqualWidth => include_bytes!("../icons/lucide/stretch-horizontal.svg"),
        Command::EqualHeight => include_bytes!("../icons/lucide/stretch-vertical.svg"),
        Command::EqualSize => include_bytes!("../icons/lucide/scaling.svg"),
        Command::DistributeAcross => {
            include_bytes!("../icons/lucide/align-horizontal-distribute-center.svg")
        }
        // À lâmina (spec GJ): os mesmos ícones; o rótulo distingue.
        Command::AlignLeftToSpread => include_bytes!("../icons/lucide/align-start-vertical.svg"),
        Command::AlignCenterToSpread => {
            include_bytes!("../icons/lucide/align-center-vertical.svg")
        }
        Command::AlignRightToSpread => include_bytes!("../icons/lucide/align-end-vertical.svg"),
        Command::AlignTopToSpread => {
            include_bytes!("../icons/lucide/align-start-horizontal.svg")
        }
        Command::AlignMiddleToSpread => {
            include_bytes!("../icons/lucide/align-center-horizontal.svg")
        }
        Command::AlignBottomToSpread => {
            include_bytes!("../icons/lucide/align-end-horizontal.svg")
        }
        Command::DistributeAcrossToSpread => {
            include_bytes!("../icons/lucide/align-horizontal-distribute-center.svg")
        }
        _ => include_bytes!("../icons/lucide/align-vertical-distribute-center.svg"),
    }
}

/// As linhas de um menu da barra: o ícone, o nome do comando e se ele vale agora.
fn rows(app: &App, menu: BarMenu) -> Vec<BarRow> {
    menu.commands()
        .iter()
        .map(|c| BarRow {
            // Um SVG embutido: se não ler, a linha fica só com o nome.
            icon: slint::Image::load_from_svg_data(icon_svg(*c)).unwrap_or_default(),
            label: c.label().into(),
            command: c.name().into(),
            enabled: app.editor.available(*c),
        })
        .collect()
}

/// Põe a ferramenta, o zoom, as dicas e o que vale agora na barra do modo organizar.
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<PaletteUi>();
    ui.set_tool(app.editor.tool().index());
    ui.set_zoom(app.editor.zoom_label().into());
    let tips: Vec<SharedString> = app
        .editor
        .palette_tips()
        .into_iter()
        .map(Into::into)
        .collect();
    ui.set_tips(crate::convert::model(tips));
    ui.set_can_align(app.editor.available(Command::AlignLeft));
    ui.set_can_distribute(app.editor.available(Command::DistributeAcross));
    let organize = w.global::<OrganizeUi>();
    let selected = app.editor.selected_frames().len();
    organize.set_can_fields(bar_groups(selected).fields);
    let menu_tips: Vec<SharedString> = BAR_MENUS_ORDER
        .iter()
        .map(|m| m.tip(selected).into())
        .collect();
    organize.set_menu_tips(crate::convert::model(menu_tips));
    organize.set_align_rows(crate::convert::model(rows(app, BarMenu::Align)));
    organize.set_equal_rows(crate::convert::model(rows(app, BarMenu::Equal)));
    organize.set_distribute_rows(crate::convert::model(rows(app, BarMenu::Distribute)));
    #[allow(clippy::cast_possible_truncation)]
    organize.set_zoom_position(app.editor.view_zoom_position() as f32);
}
