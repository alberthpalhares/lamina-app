//! Abrir, criar e salvar o álbum pela IA (spec IA, regras 5 a 7): só nas pastas permitidas, sem
//! perder alterações sem pedir e sem gravar por cima de um arquivo que mudou no disco.

use std::path::{Path, PathBuf};

use lamina_core::{ExportPrefs, PageLimits};
use lamina_layout::Reuse;
use serde_json::json;

use super::paths::{ensure_parent, lamina_file, same_file};
use super::{
    Agent, NewAlbumCall, OpenAlbum, SaveAlbum, ToolError, ToolReply, session, session_mut,
};
use crate::{Editor, NewAlbum, SpecFields, Step, clean_album_name, create_album};

/// `nome` vazio no `novo_album` e no `renomear_album`.
pub const EMPTY_NAME: &str = "O nome do álbum não pode ficar vazio.";

/// `laminas` sem `montar` no `novo_album` (spec BR, regra 14).
pub const SPREADS_NEED_BUILD: &str = "O número de lâminas só vale com montar: true. Sem montar, \
                                      o álbum começa em branco; use montar_album depois.";

impl Agent {
    /// Abre `file` ao começar (`lamina mcp <álbum>`): a pasta dele passa a ser permitida.
    pub fn start_with(&mut self, editor: &mut Editor, file: &Path) -> Result<(), ToolError> {
        if let Some(folder) = std::path::absolute(file)
            .ok()
            .and_then(|f| f.parent().map(Path::to_path_buf))
        {
            self.roots.add(&folder);
        }
        self.open_album(
            editor,
            &OpenAlbum {
                caminho: file.display().to_string(),
                descartar: true,
            },
        )
        .map(|_| ())
    }

    pub(super) fn open_album(
        &mut self,
        editor: &mut Editor,
        a: &OpenAlbum,
    ) -> Result<ToolReply, ToolError> {
        let file = self.roots.check(Path::new(&a.caminho))?;
        refuse_unsaved(editor, a.descartar)?;
        editor.open(&file)?;
        self.after_load(editor, &file);
        let s = session(editor)?;
        let text = format!("Álbum {} aberto: {}.", s.title(), s.counter());
        let missing = s.missing_message();
        Ok(ToolReply::new(text, json!({ "fotos_faltando": missing })))
    }

    pub(super) fn new_album(
        &mut self,
        editor: &mut Editor,
        a: &NewAlbumCall,
        progress: super::Progress<'_>,
    ) -> Result<ToolReply, ToolError> {
        // O número de lâminas só vale montando (spec BR, regra 14): nada é criado.
        if a.laminas.is_some() && !a.montar {
            return Err(ToolError::Refused(SPREADS_NEED_BUILD.to_owned()));
        }
        let folder = self.roots.check(Path::new(&a.pasta_fotos))?;
        let target = self.roots.check(&lamina_file(&a.salvar_em))?;
        if target.exists() {
            return Err(ToolError::FileExists(target.display().to_string()));
        }
        refuse_unsaved(editor, a.descartar)?;
        let new = self.new_request(editor, folder.clone(), a)?;
        ensure_parent(&target)?;
        let step = |s: Step| {
            if let Step::Importing { done, total } = s {
                progress(done, total);
            }
        };
        let (built, failures) = create_album(&new, &self.cache, &step)?;
        editor.adopt(built.project);
        session_mut(editor)?.save_as(&target)?;
        self.roots.add(&folder);
        self.after_load(editor, &target);
        let s = session(editor)?;
        let text = format!("Álbum {} criado e salvo: {}.", s.title(), s.counter());
        let fails: Vec<String> = failures
            .iter()
            .map(|(p, why)| format!("{}: {why}", p.display()))
            .collect();
        Ok(ToolReply::new(
            text,
            json!({ "arquivo": target, "fotos_nao_importadas": fails }),
        ))
    }

    /// O pedido igual ao do "Novo álbum" da janela com as mesmas escolhas (IA-B14).
    fn new_request(
        &self,
        editor: &Editor,
        folder: PathBuf,
        a: &NewAlbumCall,
    ) -> Result<NewAlbum, ToolError> {
        let (fields, limits, export) = match &a.preset {
            Some(name) => {
                let p = self.preset(name)?;
                (p.fields.clone(), p.limits, p.export)
            }
            None => (
                SpecFields::default(),
                PageLimits::default(),
                ExportPrefs::default(),
            ),
        };
        let spec = fields
            .to_spec()
            .map_err(|e| ToolError::Refused(e.to_string()))?;
        Ok(NewAlbum {
            name: Some(album_name(a)?),
            folder,
            spec,
            spreads: a.laminas,
            reuse: Reuse::Medium,
            preferences: editor.preferences().clone(),
            // A IA não lê o aprendizado (spec AL, regra 17).
            learned: lamina_layout::Learned::default(),
            models: editor.models().to_vec(),
            limits,
            export,
            style: Default::default(),
            files: Vec::new(),
            build: a.montar,
        })
    }

    pub(super) fn save_album(
        &mut self,
        editor: &mut Editor,
        a: &SaveAlbum,
    ) -> Result<ToolReply, ToolError> {
        let current = session(editor)?.file().map(Path::to_path_buf);
        let target = match &a.caminho {
            Some(path) => self.roots.check(&lamina_file(path))?,
            None => current.clone().ok_or(ToolError::NoFile)?,
        };
        let same = current.as_deref().is_some_and(|c| same_file(c, &target));
        if same {
            self.refuse_changed(&target)?;
        } else if target.exists() && !a.substituir {
            return Err(ToolError::FileExists(target.display().to_string()));
        }
        ensure_parent(&target)?;
        let saved = session_mut(editor)?.save_as(&target)?;
        self.disk = std::fs::read(&saved).ok();
        Ok(ToolReply::new(
            format!("Álbum salvo em {}.", saved.display()),
            json!({ "arquivo": saved }),
        ))
    }

    /// Recusa se o arquivo no disco não é mais o que foi aberto ou salvo (regra 6).
    fn refuse_changed(&self, file: &Path) -> Result<(), ToolError> {
        let now = std::fs::read(file).ok();
        if now.is_some() && now != self.disk {
            return Err(ToolError::ChangedOnDisk(file.display().to_string()));
        }
        Ok(())
    }

    /// Outro álbum na tela: as listas de layout antigas não valem mais; guarda o arquivo como
    /// está no disco.
    fn after_load(&mut self, editor: &Editor, file: &Path) {
        self.lists.clear();
        let saved = editor.session().and_then(|s| s.file()).unwrap_or(file);
        self.disk = std::fs::read(saved).ok();
    }
}

fn refuse_unsaved(editor: &Editor, discard: bool) -> Result<(), ToolError> {
    match editor.session() {
        Some(s) if s.is_dirty() && !discard => Err(ToolError::Unsaved(s.title().to_owned())),
        _ => Ok(()),
    }
}

/// O nome do álbum novo: `nome`, limpo como na janela, ou o nome do arquivo pedido (sem
/// `.lamina`), que é o que o fotógrafo chamou o álbum; o da pasta das fotos só em último caso.
fn album_name(a: &NewAlbumCall) -> Result<String, ToolError> {
    if let Some(nome) = &a.nome {
        return clean_album_name(nome).ok_or_else(|| ToolError::Refused(EMPTY_NAME.to_owned()));
    }
    let file = lamina_file(&a.salvar_em);
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
    Ok(stem
        .as_deref()
        .and_then(clean_album_name)
        .unwrap_or_else(|| crate::folder_name(Path::new(&a.pasta_fotos))))
}

/// `renomear_album`: a mesma edição da janela (spec NA), com as recusas em frase.
pub(super) fn rename_album(editor: &mut Editor, nome: &str) -> Result<ToolReply, ToolError> {
    let clean = clean_album_name(nome).ok_or_else(|| ToolError::Refused(EMPTY_NAME.to_owned()))?;
    if session(editor)?.project().name == clean {
        return Err(ToolError::Refused(format!("O álbum já se chama {clean}.")));
    }
    editor.rename_album(&clean);
    let s = session(editor)?;
    Ok(ToolReply::new(
        format!("Álbum renomeado: {clean}. O arquivo .lamina continua com o mesmo nome."),
        json!({ "nome": clean, "alteracoes_nao_salvas": s.is_dirty() }),
    ))
}
