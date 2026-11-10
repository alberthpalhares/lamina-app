//! As duas esperas da spec FB (regras 6 e 7), com as contas do editor: o silêncio depois de a
//! janela mudar de tamanho (a lâmina não anima e a grade de fotos redesenha uma vez no fim) e a
//! barra da lâmina, que fica 500 ms depois que o ponteiro sai.

use std::time::{Duration, Instant};

use lamina_editor::motion::{HeaderClicks, HoverGrace, Quiet, RESIZE_QUIET_MS};
use slint::ComponentHandle;

use crate::state::App;
use crate::ui::{Motion, RollUi};

pub(crate) struct Waits {
    origin: Instant,
    resize: Quiet,
    bar: HoverGrace,
    headers: HeaderClicks,
    /// A janela recém-criada ainda entrega o primeiro Resized (o tamanho inicial/maximizado):
    /// não é o fotógrafo mudando o tamanho, e o silêncio de 150 ms atrasava o primeiro quadro
    /// completo da abertura (conferência do dono 2026-11-10, "o maximizar ainda demora").
    opening: bool,
}

impl Default for Waits {
    fn default() -> Self {
        Waits {
            origin: Instant::now(),
            resize: Quiet::new(RESIZE_QUIET_MS),
            bar: HoverGrace::default(),
            headers: HeaderClicks::default(),
            opening: true,
        }
    }
}

impl Waits {
    fn now(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Um clique no cabeçalho da faixa `strip` agora conta? O segundo toque de um clique duplo,
    /// não (spec FX, regra 9).
    pub(crate) fn header_click(&mut self, strip: u8) -> bool {
        let now = self.now();
        self.headers.accept(strip, now)
    }

    /// Ainda dentro de uma mudança de tamanho da janela.
    pub(crate) fn resizing(&self) -> bool {
        self.resize.busy(self.now())
    }
}

/// Até a hora `due` (em ms desde `origin`), contada de agora, com um fio de folga.
fn until(waits: &Waits, due: u64) -> Duration {
    Duration::from_millis(due.saturating_sub(waits.now()) + 2)
}

/// A janela mudou de tamanho: a lâmina para de animar até o silêncio. O primeiro Resized (o da
/// criação, maximizada) não conta: a abertura já desenha tudo e o silêncio só atrasaria o
/// primeiro quadro.
pub(crate) fn resized(app: &mut App) {
    if app.waits.opening {
        app.waits.opening = false;
        return;
    }
    let now = app.waits.now();
    app.waits.resize.poke(now);
    if let Some(w) = app.window.upgrade() {
        w.global::<Motion>().set_resizing(true);
    }
    if let Some(due) = app.waits.resize.due() {
        crate::mailbox::later(until(&app.waits, due), settle);
    }
}

/// O silêncio chegou (uma vez por sequência): anima de novo e redesenha a grade de fotos.
fn settle(app: &mut App) {
    let now = app.waits.now();
    if !app.waits.resize.settle(now) {
        return;
    }
    if let Some(w) = app.window.upgrade() {
        w.global::<Motion>().set_resizing(false);
    }
    app.fit_thumbs();
    app.show_grid(false);
    // A janela que encolheu só muda a largura mostrada do painel (spec FX, regra 15).
    crate::panel_width::refresh(app);
}

/// O ponteiro chegou perto da barra da lâmina ou saiu de perto dela.
pub(crate) fn bar_near(app: &mut App, near: bool) {
    let now = app.waits.now();
    app.waits.bar.set(near, now);
    show_bar(app);
    if let Some(due) = app.waits.bar.due() {
        crate::mailbox::later(until(&app.waits, due), show_bar);
    }
}

fn show_bar(app: &mut App) {
    let held = app.waits.bar.shown(app.waits.now(), false);
    if let Some(w) = app.window.upgrade() {
        w.global::<RollUi>().set_bar_held(held);
    }
}
