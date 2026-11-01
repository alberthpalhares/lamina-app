//! O "Experimente agora" da página de novidades (spec PV, regra 14).

use super::Editor;
use crate::Command;
use crate::updates::news::{NewsItem, NewsSystem, TryNow};

impl Editor {
    /// O botão da novidade agora: escondido, pronto ou apagado com o motivo.
    pub fn try_now(&self, item: &NewsItem) -> TryNow {
        let here = item
            .only
            .is_none_or(|system| system == NewsSystem::current());
        let command = item.command.as_deref().and_then(Command::from_name);
        let Some(command) = command.filter(|_| here) else {
            return TryNow::Hidden;
        };
        if self.available(command) {
            TryNow::Ready
        } else if self.session.is_none() {
            TryNow::Unavailable("Abra um álbum para experimentar.".to_owned())
        } else {
            TryNow::Unavailable("Não dá para experimentar agora.".to_owned())
        }
    }
}
