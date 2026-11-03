//! O trabalhador das fotos de tela (spec MV, regra 1): lê do cache e prepara os pixels fora da
//! linha da janela, na ordem do plano do editor (`ScreenPlan`). Quando a lâmina muda, os pedidos
//! que ainda não começaram são trocados pelos novos.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use lamina_core::{Photo, PhotoId};
use lamina_editor::{Cache, load_screen};
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::mailbox::Mailbox;
use crate::state::Msg;

/// Linhas que leem fotos ao mesmo tempo.
const READERS: usize = 4;

#[derive(Debug, Default)]
struct Queue {
    /// O álbum (`epoch`) e a lâmina dos pedidos na fila.
    owner: Option<(u64, usize)>,
    items: VecDeque<(PhotoId, Photo)>,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

/// A fila de pedidos e as linhas que a atendem.
#[derive(Debug, Clone)]
pub struct Loader {
    queue: Shared,
}

impl Loader {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        let queue: Shared = Arc::default();
        for _ in 0..READERS {
            let (queue, cache, mailbox) = (queue.clone(), cache.clone(), mailbox.clone());
            std::thread::spawn(move || serve(&queue, &cache, &mailbox));
        }
        Loader { queue }
    }

    /// Pede as fotos da lâmina `center` do álbum `epoch`. Pedidos de outra lâmina que ainda não
    /// começaram saem da fila (regra 1).
    /// Nada na fila (a filmagem espera por isto).
    #[cfg(any(test, feature = "filmagem"))]
    pub fn idle(&self) -> bool {
        let queue = self.queue.0.lock().unwrap_or_else(PoisonError::into_inner);
        queue.items.is_empty()
    }

    pub fn ask(&self, epoch: u64, center: usize, items: Vec<(PhotoId, Photo)>) {
        let (lock, wake) = &*self.queue;
        let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if q.owner != Some((epoch, center)) {
            q.owner = Some((epoch, center));
            q.items.clear();
        }
        q.items.extend(items);
        wake.notify_all();
    }
}

/// Uma linha de leitura: pega o próximo pedido, lê e manda para a janela.
fn serve(queue: &Shared, cache: &Cache, mailbox: &Mailbox) {
    let (lock, wake) = &**queue;
    loop {
        let (epoch, id, photo) = {
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let (Some((epoch, _)), Some((id, photo))) = (q.owner, q.items.pop_front()) {
                    break (epoch, id, photo);
                }
                q = wake.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
        };
        let pixels = load_screen(&photo, cache).map(|img| {
            SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                img.as_raw(),
                img.width(),
                img.height(),
            )
        });
        // Janela fechada: ninguém para receber, e a linha acaba.
        if mailbox.send(Msg::Screen { epoch, id, pixels }).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedido_de_outra_lamina_troca_a_fila() {
        let loader = Loader {
            queue: Arc::default(),
        };
        let spec = lamina_core::AlbumSpec::new(
            lamina_core::Mm::from_cm(30.0),
            lamina_core::Mm::from_cm(30.0),
            lamina_core::Dpi::new(300).unwrap(),
        )
        .unwrap();
        let mut project = lamina_core::Project::new("teste", spec);
        let ids: Vec<PhotoId> = (0..3)
            .map(|i| project.add_photo(format!("{i}.jpg"), 10, 10))
            .collect();
        let item = |i: usize| (ids[i], project.photo(ids[i]).unwrap().clone());
        loader.ask(1, 3, vec![item(0)]);
        loader.ask(1, 3, vec![item(1)]);
        assert_eq!(loader.queue.0.lock().unwrap().items.len(), 2);
        // Outra lâmina: o que não começou sai da fila.
        loader.ask(1, 7, vec![item(2)]);
        let q = loader.queue.0.lock().unwrap();
        assert_eq!(q.items.len(), 1);
        assert_eq!(q.items[0].0, ids[2]);
        assert_eq!(q.owner, Some((1, 7)));
    }
}
