//! Toca a animação de uma novidade na própria capa, com um clique (spec PV, regra 9). O que cada
//! quadro mostra e quando o laço volta sai do `lamina_editor::updates::news::Animation`; aqui só o
//! temporizador e a troca da imagem na janela.

use std::cell::RefCell;
use std::time::Duration;

use lamina_editor::updates::news::{Animation, Illustration};
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::{AppWindow, NewsUi};

/// O item que toca, a animação dele e a geração: tocar outro item ou parar muda a geração, e o quadro
/// agendado da anterior não troca mais nada.
#[derive(Default)]
struct Player {
    item: Option<usize>,
    generation: u64,
    animation: Option<Animation<'static>>,
}

thread_local! {
    static PLAYER: RefCell<Player> = RefCell::new(Player::default());
}

/// A imagem da janela a partir de um quadro decodificado. Uma cópia só, de bytes: a cópia pixel a
/// pixel levava ~8 ms por quadro no Lamina compilado em debug.
pub fn image(picture: &Illustration) -> Image {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(picture.width, picture.height);
    let bytes = buffer.make_mut_bytes();
    if bytes.len() != picture.rgba.len() {
        return Image::default();
    }
    bytes.copy_from_slice(&picture.rgba);
    Image::from_rgba8(buffer)
}

/// O item que toca agora: identifica o item, não a linha, porque com filtro as linhas mudam.
pub fn playing() -> Option<usize> {
    PLAYER.with_borrow(|p| p.item)
}

/// Um clique na capa do item: toca; se ele já toca, para. Só uma toca por vez.
pub fn toggle(window: &AppWindow, item: usize, bytes: &'static [u8]) {
    let again = playing() == Some(item);
    stop(window);
    if again {
        return;
    }
    let started = PLAYER.with_borrow_mut(|p| {
        let animation = Animation::open(bytes, false).filter(Animation::animated)?;
        let first = animation.first().clone();
        p.animation = Some(animation);
        p.item = Some(item);
        Some((p.generation, first))
    });
    if let Some((generation, first)) = started {
        let ui = window.global::<NewsUi>();
        ui.set_frame(image(&first.picture));
        ui.set_playing(i32::try_from(item).unwrap_or(-1));
        schedule(window.as_weak(), generation, first.delay_ms);
    }
}

/// Para de tocar e solta o quadro da memória: a capa volta ao primeiro quadro.
pub fn stop(window: &AppWindow) {
    forget();
    let ui = window.global::<NewsUi>();
    ui.set_playing(-1);
    ui.set_frame(Image::default());
}

fn forget() {
    PLAYER.with_borrow_mut(|p| {
        p.generation += 1;
        p.item = None;
        p.animation = None;
    });
}

fn schedule(window: slint::Weak<AppWindow>, generation: u64, delay_ms: u32) {
    slint::Timer::single_shot(Duration::from_millis(u64::from(delay_ms)), move || {
        tick(&window, generation);
    });
}

/// Troca para o quadro seguinte e agenda o próximo; fechar a página para o laço.
fn tick(window: &slint::Weak<AppWindow>, generation: u64) {
    let Some(w) = window.upgrade() else {
        return forget();
    };
    let ui = w.global::<NewsUi>();
    if !ui.get_show() {
        return stop(&w);
    }
    let frame = PLAYER.with_borrow_mut(|p| {
        if p.generation != generation {
            return None;
        }
        p.animation.as_mut().and_then(Animation::advance)
    });
    let Some(frame) = frame else {
        return;
    };
    ui.set_frame(image(&frame.picture));
    schedule(window.clone(), generation, frame.delay_ms);
}
