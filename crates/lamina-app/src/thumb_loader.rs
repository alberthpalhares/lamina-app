//! O trabalhador das miniaturas (spec TR, regra 6): lê do cache fora da linha da janela, na ordem
//! em que as listas pedem (`ThumbPlan` do editor). Um álbum novo esvazia a fila.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

use lamina_core::{Photo, PhotoId};
use lamina_editor::{Cache, load_thumb_sized};
use slint::{Rgb8Pixel, SharedPixelBuffer};

use crate::mailbox::Mailbox;
use crate::state::Msg;

/// Linhas que leem miniaturas ao mesmo tempo: cada uma leva ~2 ms, e as imagens de tela têm as
/// delas.
const READERS: usize = 2;

pub type ThumbPixels = SharedPixelBuffer<Rgb8Pixel>;

#[derive(Debug, Default)]
struct Queue {
    /// O álbum (`epoch`) dos pedidos na fila.
    epoch: u64,
    items: VecDeque<(PhotoId, Photo)>,
    /// O lado das miniaturas lidas, em px (0 = o do disco; spec PF, seção 13).
    side: u32,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

/// A fila de pedidos e as linhas que a atendem.
#[derive(Debug, Clone)]
pub struct ThumbLoader {
    queue: Shared,
}

impl ThumbLoader {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        let queue: Shared = Arc::default();
        for _ in 0..READERS {
            let (queue, cache, mailbox) = (queue.clone(), cache.clone(), mailbox.clone());
            std::thread::spawn(move || serve(&queue, &cache, &mailbox));
        }
        ThumbLoader { queue }
    }

    /// Um `ThumbLoader` sem as linhas de leitura (testes: nenhuma leitura sai).
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        ThumbLoader {
            queue: Arc::default(),
        }
    }

    /// Pede as miniaturas `items` do álbum `epoch`; pedidos de outro álbum saem da fila.
    pub fn ask(&self, epoch: u64, items: Vec<(PhotoId, Photo)>) {
        if items.is_empty() {
            return;
        }
        let (lock, wake) = &*self.queue;
        let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if q.epoch != epoch {
            q.epoch = epoch;
            q.items.clear();
        }
        q.items.extend(items);
        wake.notify_all();
    }

    /// O lado das próximas miniaturas lidas (o tamanho da peça da grade; spec PF).
    pub fn set_side(&self, side: u32) {
        let (lock, _) = &*self.queue;
        lock.lock().unwrap_or_else(PoisonError::into_inner).side = side;
    }

    /// Tira da fila os pedidos `ids` ainda não lidos (a grade rolou para longe; spec PF).
    pub fn cancel(&self, ids: &[PhotoId]) {
        if ids.is_empty() {
            return;
        }
        let gone: std::collections::HashSet<PhotoId> = ids.iter().copied().collect();
        let (lock, _) = &*self.queue;
        let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        q.items.retain(|(id, _)| !gone.contains(id));
    }
}

/// Uma linha de leitura: pega o próximo pedido, lê e manda para a janela.
fn serve(queue: &Shared, cache: &Cache, mailbox: &Mailbox) {
    let (lock, wake) = &**queue;
    loop {
        let (epoch, id, photo, side) = {
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let Some((id, photo)) = q.items.pop_front() {
                    break (q.epoch, id, photo, q.side);
                }
                q = wake.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
        };
        let side = if side == 0 { u32::MAX } else { side };
        let pixels = load_thumb_sized(&photo, cache, side)
            .map(|img| ThumbPixels::clone_from_slice(img.as_raw(), img.width(), img.height()));
        // Janela fechada: ninguém para receber, e a linha acaba.
        if mailbox.send(Msg::Thumb { epoch, id, pixels }).is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedido_de_outro_album_esvazia_a_fila() {
        let loader = ThumbLoader {
            queue: Arc::default(),
        };
        let cm = lamina_core::Mm::from_cm(30.0);
        let spec =
            lamina_core::AlbumSpec::new(cm, cm, lamina_core::Dpi::new(300).unwrap()).unwrap();
        let mut project = lamina_core::Project::new("teste", spec);
        let ids: Vec<PhotoId> = (0..3)
            .map(|i| project.add_photo(format!("{i}.jpg"), 10, 10))
            .collect();
        let item = |i: usize| (ids[i], project.photo(ids[i]).unwrap().clone());
        loader.ask(0, vec![item(0), item(1)]);
        loader.ask(0, Vec::new());
        assert_eq!(loader.queue.0.lock().unwrap().items.len(), 2);
        loader.ask(1, vec![item(2)]);
        let q = loader.queue.0.lock().unwrap();
        assert_eq!((q.epoch, q.items.len()), (1, 1));
        assert_eq!(q.items[0].0, ids[2]);
    }

    #[test]
    fn pedido_cancelado_sai_da_fila() {
        let loader = ThumbLoader {
            queue: Arc::default(),
        };
        let cm = lamina_core::Mm::from_cm(30.0);
        let spec =
            lamina_core::AlbumSpec::new(cm, cm, lamina_core::Dpi::new(300).unwrap()).unwrap();
        let mut project = lamina_core::Project::new("teste", spec);
        let ids: Vec<PhotoId> = (0..3)
            .map(|i| project.add_photo(format!("{i}.jpg"), 10, 10))
            .collect();
        let items = ids
            .iter()
            .map(|id| (*id, project.photo(*id).unwrap().clone()))
            .collect();
        loader.ask(0, items);
        loader.cancel(&[ids[0], ids[2]]);
        let q = loader.queue.0.lock().unwrap();
        let left: Vec<PhotoId> = q.items.iter().map(|(id, _)| *id).collect();
        assert_eq!(left, [ids[1]]);
    }
}
