//! Modelos do fotógrafo pelo editor (spec MD): o arquivo dos modelos, salvar a lâmina atual,
//! renomear e apagar levando junto as marcas de favorito e rejeitado.

use lamina_layout::{SpreadModel, model_key};

use super::Editor;
use crate::models::{ModelError, ModelsStore, model_from_spread};

impl Editor {
    pub fn models(&self) -> &[SpreadModel] {
        self.models.all()
    }

    /// Usa os modelos deste arquivo; o aviso de leitura, se houver, vira aviso da janela.
    pub fn set_models(&mut self, mut store: ModelsStore) {
        if let Some(warning) = store.warning.take() {
            self.notice = Some(warning);
        }
        self.models = store;
        self.lists.borrow_mut().clear();
    }

    pub fn model_exists(&self, name: &str) -> bool {
        self.models.exists(name)
    }

    /// A lâmina atual pode virar modelo? (tem quadro de foto; regra 1)
    pub fn can_save_model(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|s| model_from_spread(s.project(), s.current(), "x").is_ok())
    }

    /// Salva a lâmina atual como o modelo `name` (substitui um de mesmo nome).
    pub fn save_model(&mut self, name: &str) -> Result<(), ModelError> {
        let s = self.session.as_ref().ok_or(ModelError::NoPhotoFrame)?;
        let model = model_from_spread(s.project(), s.current(), name)?;
        self.models.save(model)?;
        self.lists.borrow_mut().clear();
        Ok(())
    }

    /// Renomeia o modelo; as marcas de favorito e rejeitado vão junto (regra 5).
    pub fn rename_model(&mut self, old: &str, new: &str) -> Result<bool, ModelError> {
        if !self.models.rename(old, new)? {
            return Ok(false);
        }
        let prefs = &mut self.prefs.prefs;
        for mirrored in [false, true] {
            let (from, to) = (
                model_key(old, mirrored, false),
                model_key(new.trim(), mirrored, false),
            );
            for set in [&mut prefs.favorites, &mut prefs.rejected] {
                if set.remove(&from) {
                    set.insert(to.clone());
                }
            }
        }
        self.after_models_change();
        Ok(true)
    }

    /// Oculta um modelo de fábrica (spec RM, regra 8): sai da faixa e do painel; restaurar traz
    /// de volta. `true` = mudou.
    pub fn hide_factory_model(&mut self, name: &str) -> bool {
        for mirrored in [false, true] {
            let key = model_key(name, mirrored, true);
            self.prefs.prefs.hide(&key);
        }
        self.lists.borrow_mut().clear();
        self.save_prefs();
        true
    }

    /// Restaura os modelos de fábrica (spec RM, regra 7). `true` = havia algo a restaurar.
    pub fn restore_factory_models(&mut self) -> bool {
        let had = !self.prefs.prefs.hidden.is_empty()
            || self
                .prefs
                .prefs
                .favorites
                .iter()
                .any(|k| k.starts_with("fabrica:"))
            || self
                .prefs
                .prefs
                .rejected
                .iter()
                .any(|k| k.starts_with("fabrica:"));
        self.prefs.prefs.restore_factory();
        self.lists.borrow_mut().clear();
        self.save_prefs();
        had
    }

    /// Há modelos de fábrica ocultos ou marcados (o "Restaurar" vale)?
    pub fn can_restore_factory(&self) -> bool {
        !self.prefs.prefs.hidden.is_empty()
            || self
                .prefs
                .prefs
                .favorites
                .iter()
                .any(|k| k.starts_with("fabrica:"))
            || self
                .prefs
                .prefs
                .rejected
                .iter()
                .any(|k| k.starts_with("fabrica:"))
    }

    /// Um nome de cópia de fábrica já existe (entre os meus e na fábrica; spec RM, regra 8)?
    pub fn factory_copy_exists(&self, name: &str) -> bool {
        self.model_exists(name)
            || lamina_layout::factory::factory_models_all()
                .iter()
                .any(|m| m.name == name)
    }

    /// Salva o desenho do modelo de fábrica `source` como um modelo do fotógrafo `name` (spec RM,
    /// regra 8: "Salvar como cópia…").
    pub fn save_factory_copy(&mut self, source: &str, name: &str) -> Result<(), ModelError> {
        let found = lamina_layout::factory::factory_models_all()
            .into_iter()
            .find(|m| m.name == source)
            .ok_or(ModelError::NotFound)?;
        let mut copy = found.clone();
        copy.factory = false;
        copy.name = name.trim().to_owned();
        self.models.save(copy)?;
        self.lists.borrow_mut().clear();
        Ok(())
    }

    /// Apaga o modelo e as marcas dele.
    pub fn remove_model(&mut self, name: &str) -> Result<bool, ModelError> {
        if !self.models.remove(name)? {
            return Ok(false);
        }
        let prefs = &mut self.prefs.prefs;
        for mirrored in [false, true] {
            let key = model_key(name, mirrored, false);
            prefs.favorites.remove(&key);
            prefs.rejected.remove(&key);
        }
        self.after_models_change();
        Ok(true)
    }

    fn after_models_change(&mut self) {
        self.lists.borrow_mut().clear();
        if let Err(message) = self.prefs.save() {
            self.notice = Some(message);
        }
    }
}
