//! O menu de contexto da lâmina (spec EG, regra 13): os itens saem do registro de comandos, com a
//! mesma disponibilidade do menu principal. Clique direito num quadro fora da seleção seleciona
//! só ele; dentro, mantém a seleção. Vale também fora do modo organizar.

use lamina_core::FrameContent;

use super::Editor;
use crate::Command;
use crate::leaving::remove_label;

/// Um item do menu de contexto: um comando (com o rótulo e se vale agora), um submenu ou um
/// separador.
#[derive(Debug, Clone, PartialEq)]
pub enum ContextEntry {
    Item {
        command: Command,
        label: String,
        /// O atalho do registro, para mostrar à direita; vazio, sem atalho (spec DC, regra 2).
        shortcut: String,
        enabled: bool,
        /// O item mostra a marca (a página já está em branco; spec PB).
        checked: bool,
    },
    Sub {
        title: &'static str,
        items: Vec<ContextEntry>,
    },
    Separator,
}

const ALIGN: [Command; 6] = [
    Command::AlignLeft,
    Command::AlignCenter,
    Command::AlignRight,
    Command::AlignTop,
    Command::AlignMiddle,
    Command::AlignBottom,
];
const EQUAL: [Command; 3] = [
    Command::EqualWidth,
    Command::EqualHeight,
    Command::EqualSize,
];
const DISTRIBUTE: [Command; 2] = [Command::DistributeAcross, Command::DistributeDown];

impl Editor {
    /// Clique direito num quadro (`Some`) ou no fundo (`None`): ajusta a seleção e devolve o menu.
    pub fn context_menu(&mut self, frame: Option<usize>) -> Vec<ContextEntry> {
        match frame {
            Some(f) => {
                self.context_side = None;
                if !self.selected_frames().contains(&f) {
                    self.exit_crop();
                    self.organize_click(f, false, false);
                }
                self.frame_menu()
            }
            None => self.background_menu(),
        }
    }

    fn entry(&self, command: Command) -> ContextEntry {
        ContextEntry::Item {
            command,
            label: self.context_label(command),
            shortcut: command.shortcut().map(|s| s.text()).unwrap_or_default(),
            enabled: self.available(command) && command.in_mode(self.mode()),
            checked: command == Command::BlankPage && self.context_page_is_blank(),
        }
    }

    fn sub(&self, title: &'static str, commands: &[Command]) -> ContextEntry {
        ContextEntry::Sub {
            title,
            items: commands.iter().map(|c| self.entry(*c)).collect(),
        }
    }

    /// O rótulo do registro; Delete diz o que vai fazer, e Organizar vira "Voltar" no modo (spec ZR).
    fn context_label(&self, command: Command) -> String {
        match command {
            Command::Organize if self.organizing() => "Voltar".to_owned(),
            Command::ClearFrame => {
                let photo = self.selected_frames().iter().any(|f| {
                    self.session.as_ref().is_some_and(|s| {
                        s.project()
                            .spreads
                            .get(s.current())
                            .and_then(|spread| spread.frames.get(*f))
                            .is_some_and(|fr| matches!(fr.content, FrameContent::Photo { .. }))
                    })
                });
                if photo { "Tirar foto" } else { "Apagar quadro" }.to_owned()
            }
            Command::RemoveFromAlbum => remove_label(self.selected_frame_photos().len()),
            c => c.label().to_owned(),
        }
    }

    fn frame_menu(&self) -> Vec<ContextEntry> {
        use Command as C;
        // Num quadro com foto, o item tira a foto do álbum; no vazio, apaga o quadro (spec TF-B11).
        let clear = if self.selected_frame_photos().is_empty() {
            C::ClearFrame
        } else {
            C::RemoveFromAlbum
        };
        let mut menu: Vec<ContextEntry> = [C::EditText, C::CropMode, C::AdjustPhoto, clear]
            .iter()
            .map(|c| self.entry(*c))
            .collect();
        // Só numa foto ausente, e no topo: é o que o fotógrafo precisa fazer (spec RF, regra 8).
        if self.available(C::LocatePhoto) {
            menu.insert(0, self.entry(C::LocatePhoto));
        }
        menu.push(ContextEntry::Separator);
        for c in [C::CopyFrames, C::PasteFrames, C::DuplicateFrames] {
            menu.push(self.entry(c));
        }
        menu.push(ContextEntry::Separator);
        menu.push(self.entry(C::BringForward));
        menu.push(self.entry(C::SendBackward));
        menu.push(self.sub("Alinhar", &ALIGN));
        menu.push(self.sub("Igualar", &EQUAL));
        menu.push(self.sub("Distribuir", &DISTRIBUTE));
        menu.push(ContextEntry::Separator);
        menu.push(self.entry(C::SelectAllFrames));
        menu
    }

    fn background_menu(&self) -> Vec<ContextEntry> {
        use Command as C;
        let groups: [&[Command]; 5] = [
            &[C::PasteFrames, C::SelectAllFrames],
            &[C::NewText, C::NewPhotoFrame],
            &[C::BlankPage, C::Mirror, C::SaveModel],
            &[
                C::NewSpread,
                C::NewBlankSpread,
                C::NewBlankSpreadBefore,
                C::DeleteSpread,
            ],
            &[C::ToggleGuides, C::Organize],
        ];
        let mut menu = Vec::new();
        for (k, group) in groups.iter().enumerate() {
            if k > 0 {
                menu.push(ContextEntry::Separator);
            }
            menu.extend(group.iter().map(|c| self.entry(*c)));
        }
        menu
    }
}
