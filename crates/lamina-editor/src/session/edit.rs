//! Edições do projeto pela sessão: cada uma entra no histórico com um rótulo (ADR 0005), e
//! desfazer e refazer levam a lâmina atual até a lâmina que mudou (spec TL, regra 9).

use lamina_core::Project;
use lamina_export::preflight;

use super::{Session, missing_of, warnings_of};

/// Aplica `change` no álbum, ou na projeção da capa e devolve o resultado (spec CA, regra 4). Com
/// a lombada ou as medidas mudando, os quadros da capa acompanham na mesma edição (regra 7).
fn apply_edit<E>(
    project: &mut Project,
    canvas: Option<&Project>,
    change: impl FnOnce(&mut Project) -> Result<(), E>,
) -> Result<(), E> {
    if let Some(canvas) = canvas {
        let mut canvas = canvas.clone();
        change(&mut canvas)?;
        project.absorb_cover(canvas);
        return Ok(());
    }
    let before = project.cover.as_ref().map(|c| c.measures);
    let spine = project.cover_spine();
    change(project)?;
    let now = project.cover.as_ref().map(|c| c.measures);
    if let (Some(old), Some(old_spine)) = (before, spine)
        && (now != before || project.cover_spine() != spine)
    {
        project.refit_cover(&old, old_spine);
    }
    Ok(())
}

impl Session {
    /// Aplica `change` no projeto como uma edição com `label`. Se der erro, nada muda. O rótulo
    /// ganha a lâmina atual ("Trocar layout · lâmina 3"), salvo quando já diz onde (spec HI,
    /// regra 1).
    pub fn edit<E>(
        &mut self,
        label: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        let prefix = self.label_prefix;
        let label = if self.on_cover() {
            // Na capa, o rótulo diz "capa", mesmo quando veio com a lâmina da projeção (a IA).
            match label.split_once(" · ") {
                Some((what, place)) if place.starts_with("lâmina ") => {
                    format!("{prefix}{what} · capa")
                }
                Some(_) => format!("{prefix}{label}"),
                None => format!("{prefix}{label} · capa"),
            }
        } else if label.contains(" · ") {
            format!("{prefix}{label}")
        } else {
            format!("{prefix}{label} · lâmina {}", self.current + 1)
        };
        let before = self.album_shared();
        let canvas = self.canvas.clone();
        self.history
            .try_apply(label.clone(), |p| apply_edit(p, canvas.as_deref(), change))?;
        self.note_spine(&before, &label);
        self.rebuild_canvas();
        self.update_warnings(&before);
        self.merge = None;
        Ok(())
    }

    /// Uma edição que vale para o álbum, mesmo com a capa aberta, com o rótulo como está (criar,
    /// medir e apagar a capa; spec CA, regras 6, 7 e 12).
    pub fn edit_album<E>(
        &mut self,
        label: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        let label = format!("{}{label}", self.label_prefix);
        let before = self.album_shared();
        self.history
            .try_apply(label, |p| apply_edit(p, None, change))?;
        self.rebuild_canvas();
        self.update_warnings(&before);
        self.merge = None;
        Ok(())
    }

    /// A lombada calculada mudou nesta edição: o rótulo diz (spec CA, regra 7).
    fn note_spine(&mut self, before: &Project, label: &str) {
        let now = self.album();
        let same_measures =
            before.cover.as_ref().map(|c| c.measures) == now.cover.as_ref().map(|c| c.measures);
        let (Some(old), Some(new)) = (before.cover_spine(), now.cover_spine()) else {
            return;
        };
        if same_measures && old != new {
            let cm = |mm: lamina_core::Mm| format!("{:.1}", mm.get() / 10.0).replace('.', ",");
            let text = format!("{label} · lombada {} → {} cm", cm(old), cm(new));
            self.history.relabel_last(text);
        }
    }

    /// Como [`Session::edit`], mas emenda a edição anterior quando ela tem a mesma `key` e nada
    /// aconteceu no meio: digitar ou mover aos poucos vira um passo só (spec TX, regra 10).
    pub fn edit_merging<E>(
        &mut self,
        label: &str,
        key: &str,
        change: impl FnOnce(&mut Project) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.merge.as_deref() == Some(key) && self.history.can_undo() {
            let before = self.album_shared();
            let canvas = self.canvas.clone();
            self.history
                .try_amend(|p| apply_edit(p, canvas.as_deref(), change))?;
            self.rebuild_canvas();
            self.update_warnings(&before);
            return Ok(());
        }
        self.edit(label, change)?;
        self.merge = Some(key.to_owned());
        Ok(())
    }

    /// A próxima edição não emenda a anterior (trocou de campo, de quadro ou de comando).
    pub fn end_merge(&mut self) {
        self.merge = None;
    }

    /// As próximas edições começam o rótulo com `prefix` ("IA: "; "" volta ao normal).
    pub fn set_label_prefix(&mut self, prefix: &'static str) {
        self.label_prefix = prefix;
        self.merge = None;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Desfaz a última edição; devolve o rótulo dela.
    pub fn undo(&mut self) -> Option<String> {
        let before = self.album_shared();
        let label = self.history.undo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    /// Refaz a edição desfeita; devolve o rótulo dela.
    pub fn redo(&mut self) -> Option<String> {
        let before = self.album_shared();
        let label = self.history.redo()?.to_owned();
        self.after_step(&before);
        Some(label)
    }

    pub(super) fn after_step(&mut self, before: &Project) {
        self.merge = None;
        let now = self.history.present();
        let changed = (0..now.spreads.len().max(before.spreads.len()))
            .find(|&i| now.spreads.get(i) != before.spreads.get(i));
        let last = now.spreads.len().saturating_sub(1);
        self.current = changed.unwrap_or(self.current).min(last);
        // Desfazer uma edição da capa volta para a capa; da lâmina, para a lâmina (spec CA, regra 5).
        let cover_changed = now.cover != before.cover;
        let cover_now = now.cover.is_some();
        if changed.is_some() || !cover_now {
            self.canvas = None;
        } else if cover_changed {
            self.go_to_cover();
        } else {
            self.rebuild_canvas();
        }
        let now = self.history.present();
        // Desfazer "Reencontrar fotos" volta aos caminhos antigos (spec EI). Desfazer um ajuste de
        // brilho não muda caminho nenhum: não olha o disco (spec LV, regra 5).
        let paths = now.photos.iter().map(|p| &p.path);
        if !paths.eq(before.photos.iter().map(|p| &p.path)) {
            self.missing = missing_of(now);
        }
        self.update_warnings(before);
    }

    /// Refaz os avisos de pré-impressão: só das lâminas que mudaram quando dá (digitar num texto
    /// não desenha de novo os textos do álbum inteiro; spec LV, regra 2).
    fn update_warnings(&mut self, before: &Project) {
        let now = self.history.present();
        // A capa fora da conta das lâminas: mexer nela não confere o álbum de novo (spec LV, regra 2).
        if now.cover != before.cover {
            self.warnings.retain(|warning| warning.spread != 0);
            self.warnings.extend(lamina_export::cover_preflight(now));
            self.warnings.sort_by_key(|warning| warning.spread);
        }
        let Some(changed) = now.spreads_changed_since(before) else {
            self.warnings = warnings_of(now);
            return;
        };
        if changed.is_empty() {
            return;
        }
        self.warnings
            .retain(|warning| warning.spread == 0 || !changed.contains(&(warning.spread - 1)));
        self.warnings.extend(preflight(now, &changed));
        // `sort_by_key` é estável: os avisos de cada lâmina continuam na ordem dos quadros.
        self.warnings.sort_by_key(|warning| warning.spread);
    }
}
