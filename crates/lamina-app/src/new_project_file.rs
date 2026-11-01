//! A etapa 3 do Novo projeto (spec NP): nome, onde salvar (sem substituir arquivo nenhum), a
//! pasta das fotos ou "Começar sem fotos", o número de lâminas e o "Criar", que já grava o
//! arquivo. As contas são do `lamina-editor`.

use std::path::PathBuf;

use lamina_editor::new_project::{
    CANNOT_WRITE, NO_PHOTOS_HERE, can_write, documents_dir, empty_album, free_file,
    suggested_folder,
};
use lamina_editor::{
    Human, NewAlbum, check_spreads, count_photos, folder_name, suggestion_text, suggestion_within,
};
use slint::ComponentHandle;

use crate::files::show_error;
use crate::state::App;
use crate::{AppWindow, NewProjectUi};

type Shared = std::rc::Rc<std::cell::RefCell<App>>;

/// Etapa 3 vazia: sem pasta de fotos, nome padrão e o arquivo sugerido.
pub fn reset(app: &mut App) {
    app.new_folder = PathBuf::new();
    app.new_photos = 0;
    app.new_target = None;
    app.new_files.clear();
    if let Some(w) = app.window.upgrade() {
        let ui = w.global::<NewProjectUi>();
        ui.set_name("Álbum novo".into());
        ui.set_no_photos(false);
        // "Montar as lâminas agora" vem desmarcada toda vez (spec BR, regra 2).
        ui.set_build(false);
        ui.set_photos(0);
        ui.set_photos_text("Nenhuma pasta escolhida".into());
    }
    refresh(app);
}

/// As fotos soltas na tela inicial ficam escolhidas na etapa 3 (spec IM, regra 6).
pub fn use_files(app: &mut App, files: Vec<PathBuf>, photos: usize) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let folder = files
        .first()
        .and_then(|p| {
            if p.is_dir() {
                Some(p.as_path())
            } else {
                p.parent()
            }
        })
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    ui.set_photos_text(format!("{photos} fotos soltas na janela").into());
    ui.set_photos(i32::try_from(photos).unwrap_or(i32::MAX));
    ui.set_name(folder_name(&folder).into());
    app.new_folder = folder;
    app.new_photos = photos;
    app.new_files = files;
    if let Some(spec) = app.wizard.as_ref().and_then(|w| w.size.spec().ok()) {
        let limits = crate::new_project::limits(app).unwrap_or_default();
        ui.set_spreads(i32::try_from(suggestion_within(photos, &spec, limits)).unwrap_or(1));
    }
    refresh(app);
}

/// A última pasta usada: a do álbum aberto por último (spec TI).
fn last_folder(app: &App) -> Option<PathBuf> {
    let first = app.editor.recents().list().first()?;
    first.path.parent().map(std::path::Path::to_path_buf)
}

/// O arquivo: o escolhido em "Alterar…" ou a sugestão, nunca um que já existe (regra 8).
fn target(app: &App, name: &str) -> PathBuf {
    if let Some(chosen) = &app.new_target {
        return chosen.clone();
    }
    let photos = (app.new_photos > 0).then_some(app.new_folder.as_path());
    let folder = suggested_folder(photos, last_folder(app).as_deref(), &documents_dir());
    free_file(&folder, name)
}

/// A etapa 3 na tela: o arquivo, a sugestão de lâminas e o que impede o "Criar".
pub fn refresh(app: &App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let name = ui.get_name().trim().to_owned();
    let file = target(app, &name);
    ui.set_file(file.display().to_string().into());
    let spec = app.wizard.as_ref().and_then(|w| w.size.spec().ok());
    let limits = crate::new_project::limits(app).unwrap_or_default();
    let with_photos = !ui.get_no_photos();
    let mut error = lamina_editor::forms::album_name_error(&name).map(str::to_owned);
    if let (Some(spec), true, true) = (&spec, with_photos, app.new_photos > 0) {
        let suggested = suggestion_within(app.new_photos, spec, limits);
        ui.set_suggestion(suggestion_text(suggested).into());
        ui.set_build_text(
            format!("Montar as lâminas agora (sugestão: {suggested} lâminas)").into(),
        );
    }
    if let (Some(spec), true, true) = (&spec, with_photos && ui.get_build(), app.new_photos > 0) {
        let spreads = usize::try_from(ui.get_spreads()).unwrap_or(0);
        if let Err(e) = check_spreads(app.new_photos, spreads, spec) {
            error = error.or(Some(e.human().line()));
        }
    }
    if with_photos && app.new_photos == 0 {
        error = error.or(Some(
            "Escolha a pasta das fotos ou comece sem fotos.".to_owned(),
        ));
    }
    if !can_write(file.parent().unwrap_or(&file)) {
        error = error.or(Some(CANNOT_WRITE.to_owned()));
    }
    ui.set_create_error(error.unwrap_or_default().into());
}

/// "Escolher pasta…": conta as fotos e sugere o nome e as lâminas.
fn pick_folder(app: &mut App) {
    let answer = crate::dialogs::dialog(app, "Pasta das fotos do álbum").pick_folder();
    crate::dialogs::then(answer, |app, folder| {
        if let Some(folder) = folder {
            use_folder(app, folder.path().to_path_buf());
        }
    });
}

/// A pasta escolhida: conta as fotos e sugere o nome e as lâminas.
fn use_folder(app: &mut App, folder: std::path::PathBuf) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let photos = count_photos(&folder).unwrap_or(0);
    if photos == 0 {
        app.new_photos = 0;
        ui.set_photos(0);
        ui.set_photos_text(NO_PHOTOS_HERE.into());
        return refresh(app);
    }
    ui.set_photos_text(format!("{photos} fotos em {}", folder.display()).into());
    ui.set_photos(i32::try_from(photos).unwrap_or(i32::MAX));
    if ui.get_name() == "Álbum novo" {
        ui.set_name(folder_name(&folder).into());
    }
    app.new_folder = folder;
    app.new_photos = photos;
    app.new_files.clear();
    if let Some(spec) = app.wizard.as_ref().and_then(|w| w.size.spec().ok()) {
        let limits = crate::new_project::limits(app).unwrap_or_default();
        let n = suggestion_within(photos, &spec, limits);
        ui.set_spreads(i32::try_from(n).unwrap_or(1));
    }
    refresh(app);
}

/// "Alterar…": o fotógrafo escolhe o arquivo.
fn change_file(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let name = w.global::<NewProjectUi>().get_name().trim().to_owned();
    let answer = crate::dialogs::dialog(app, "Onde salvar o álbum")
        .add_filter("Projeto do Lamina", &["lamina"])
        .set_file_name(format!("{name}.lamina"))
        .save_file();
    crate::dialogs::then(answer, |app, path| {
        if let Some(path) = path {
            app.new_target = Some(path.path().with_extension("lamina"));
        }
        refresh(app);
    });
}

/// "Criar": grava o arquivo e abre o álbum. Com fotos, monta em segundo plano.
fn create(app: &mut App) {
    let Some(w) = app.window.upgrade() else {
        return;
    };
    let ui = w.global::<NewProjectUi>();
    let Some(wizard) = app.wizard.clone() else {
        return;
    };
    let (Ok(spec), Ok(limits)) = (wizard.size.spec(), crate::new_project::limits(app)) else {
        return;
    };
    let mode = if wizard.lay_flat {
        lamina_core::DesignMode::Spread
    } else {
        lamina_core::DesignMode::Page
    };
    let name = ui.get_name().trim().to_owned();
    let file = target(app, &name);
    let new = NewAlbum {
        folder: app.new_folder.clone(),
        name: Some(name),
        spec: spec.with_mode(mode),
        spreads: usize::try_from(ui.get_spreads()).ok(),
        reuse: wizard.reuse,
        preferences: app.editor.preferences().clone(),
        learned: app.editor.learned().clone(),
        models: app.editor.models().to_vec(),
        limits,
        export: app.new_export,
        style: wizard.style(),
        files: std::mem::take(&mut app.new_files),
        build: ui.get_build(),
    };
    ui.set_show(false);
    // "Montar o álbum…" começa com a variedade usada por último nesta sessão (spec BR, regra 9).
    w.global::<crate::BuildUi>().set_reuse(ui.get_reuse());
    app.wizard = None;
    // O cartão e o tamanho marcados na próxima vez são os deste álbum (NP regra 1, ET regra 1).
    app.editor
        .remember_format(lamina_editor::new_project::Format::of(&wizard.size.fields));
    app.editor.remember_size(wizard.size.chosen().cloned());
    if ui.get_no_photos() {
        match app.editor.adopt_saved(empty_album(&new), &file) {
            Ok(_) => {
                app.project_changed();
                crate::recents::refresh(app);
            }
            Err(e) => show_error(app, &e.human()),
        }
        return;
    }
    app.new_target = Some(file);
    crate::new_album::build(app, new);
}

pub fn connect(window: &AppWindow, app: &Shared) {
    let ui = window.global::<NewProjectUi>();
    let a = app.clone();
    ui.on_pick_folder(move || pick_folder(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_change_file(move || change_file(&mut a.borrow_mut()));
    let a = app.clone();
    ui.on_file_changed(move || refresh(&a.borrow()));
    let a = app.clone();
    ui.on_create(move || create(&mut a.borrow_mut()));
}
