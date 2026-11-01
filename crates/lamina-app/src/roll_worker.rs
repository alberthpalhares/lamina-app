//! O trabalhador do rolo de lâminas (spec RL, regra 10; ADR 0012, decisão 6): desenha as imagens
//! das lâminas fora da linha da janela, com as próprias duas linhas, e manda à janela os pixels já
//! cortados em colunas (uma coluna sobe por quadro, regra 9). A fila troca inteira a cada pedido
//! do plano; o pedido que está sendo desenhado não é feito de novo.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Instant;

use lamina_core::{PhotoId, Project};
use lamina_editor::motion::{RollAsk, RollGot};
use lamina_editor::{Cache, Human, RollPainter, guarded};
use slint::{Rgba8Pixel, SharedPixelBuffer};

use crate::mailbox::Mailbox;
use crate::state::Msg;

/// Linhas que desenham cada imagem (no G0, todas as da máquina faziam a janela perder quadros).
const THREADS: usize = 2;
/// Largura de cada coluna em pixels da imagem (resultado do G0: 350 px físicos).
pub const COLUMN_PX: u32 = 350;

/// Uma imagem pronta: o pedido, como o plano a conta, as colunas e quanto levou.
#[derive(Debug)]
pub struct Painted {
    pub got: RollGot,
    pub columns: Vec<SharedPixelBuffer<Rgba8Pixel>>,
    pub ms: f64,
}

/// O que desenhar: o álbum de agora, as fotos sumidas, a largura do lugar e os pedidos.
#[derive(Debug, Clone)]
pub struct Job {
    pub project: Arc<Project>,
    pub missing: Arc<Vec<PhotoId>>,
    /// Largura do lugar em pixels lógicos (a espessura das marcas).
    pub shown: f64,
    pub asks: Vec<RollAsk>,
}

#[derive(Debug, Default)]
struct Queue {
    job: Option<Job>,
    items: VecDeque<RollAsk>,
    busy: Option<RollAsk>,
}

type Shared = Arc<(Mutex<Queue>, Condvar)>;

/// A fila do rolo e a linha que a atende.
#[derive(Debug, Clone)]
pub struct RollWorker {
    queue: Shared,
    images: Arc<lamina_editor::RollImages>,
}

impl RollWorker {
    pub fn new(cache: &Cache, mailbox: &Mailbox) -> Self {
        let queue: Shared = Arc::default();
        let (q, painter, mailbox) = (
            queue.clone(),
            RollPainter::new(THREADS, cache.clone()),
            mailbox.clone(),
        );
        let images = painter.images();
        std::thread::spawn(move || serve(&q, &painter, &mailbox));
        RollWorker { queue, images }
    }

    /// Nada na fila nem sendo desenhado (a filmagem espera por isto).
    #[cfg(any(test, feature = "filmagem"))]
    pub fn idle(&self) -> bool {
        let queue = self.queue.0.lock().unwrap_or_else(PoisonError::into_inner);
        queue.items.is_empty() && queue.busy.is_none()
    }

    /// Outro álbum: as imagens lidas do anterior vão embora (spec LV, regra 10).
    pub fn forget_images(&self) {
        self.images.clear();
    }

    /// Troca a fila pelos pedidos de `job` (regra 7).
    pub fn replace(&self, job: Job) {
        let (lock, wake) = &*self.queue;
        let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
        q.items = job.asks.iter().copied().collect();
        q.job = Some(job);
        wake.notify_all();
    }
}

/// A linha do rolo: pega o próximo pedido, desenha, corta em colunas e manda para a janela.
fn serve(queue: &Shared, painter: &RollPainter, mailbox: &Mailbox) {
    let (lock, wake) = &**queue;
    loop {
        let (ask, job) = {
            let mut q = lock.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                if let (Some(ask), Some(job)) = (q.items.pop_front(), q.job.clone()) {
                    q.busy = Some(ask);
                    break (ask, job);
                }
                q.busy = None;
                q = wake.wait(q).unwrap_or_else(PoisonError::into_inner);
            }
        };
        let started = Instant::now();
        let painted = guarded(|| painter.paint(&job.project, (&ask, job.shown), &job.missing));
        let msg = match painted {
            Ok(Ok(image)) => {
                let (w, h) = image.pixels.dimensions();
                let columns = columns(image.pixels.as_raw(), (w, h));
                let got = RollGot {
                    ask,
                    complete: image.complete,
                    bytes: image.pixels.as_raw().len(),
                };
                let ms = started.elapsed().as_secs_f64() * 1000.0;
                Msg::SpreadImage(Box::new(Ok(Painted { got, columns, ms })))
            }
            // A falha vai para o registro de erros, sem aparecer na janela (regra 6 da spec).
            Ok(Err(e)) => Msg::SpreadImage(Box::new(Err(e.human()))),
            Err(problem) => Msg::Failed(problem),
        };
        // Janela fechada: ninguém para receber, e a linha acaba.
        if mailbox.send(msg).is_err() {
            return;
        }
    }
}

/// A imagem (`raw` em RGBA, `width` × `height`) cortada em colunas de [`COLUMN_PX`] pixels (a
/// última pode ser mais estreita).
pub fn columns(raw: &[u8], (width, height): (u32, u32)) -> Vec<SharedPixelBuffer<Rgba8Pixel>> {
    (0..width.div_ceil(COLUMN_PX))
        .map(|k| {
            let x0 = k * COLUMN_PX;
            let w = COLUMN_PX.min(width - x0);
            let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, height);
            let row = (w * 4) as usize;
            let dst = buf.make_mut_bytes();
            for y in 0..height {
                let start = ((y * width + x0) * 4) as usize;
                let at = (y * w * 4) as usize;
                dst[at..at + row].copy_from_slice(&raw[start..start + row]);
            }
            buf
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn colunas_de_350_px_com_a_ultima_mais_estreita() {
        let raw: Vec<u8> = (0..3_u32)
            .flat_map(|y| {
                (0..800_u32).flat_map(move |x| [(x % 256) as u8, y as u8, (x / 256) as u8, 255])
            })
            .collect();
        let cols = columns(&raw, (800, 3));
        let widths: Vec<u32> = cols.iter().map(SharedPixelBuffer::width).collect();
        assert_eq!(widths, vec![350, 350, 100]);
        // O primeiro pixel da segunda coluna é o pixel 350 da imagem.
        let px = cols[1].as_slice()[0];
        assert_eq!((px.r, px.b), ((350 % 256) as u8, 1));
        let last = cols[2].as_slice()[(2 * 100 + 99) as usize];
        assert_eq!((last.r, last.g), ((799 % 256) as u8, 2));
    }
}
