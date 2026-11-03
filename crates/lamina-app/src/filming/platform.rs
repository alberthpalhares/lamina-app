//! A plataforma da filmagem (spec FI, regras 2 e 3): a janela desenha num buffer, o relógio é o número
//! do quadro e as tarefas das linhas de fundo esperam numa fila, esvaziada entre os quadros.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use lamina_editor::filming::{FilmError, frame_time};
use slint::platform::{EventLoopProxy, Platform, WindowAdapter};
use slint::{EventLoopError, PlatformError, Rgb8Pixel};

use crate::headless::{self, Screen};

/// O relógio: em que quadro estamos, contando os que não vão para o vídeo.
#[derive(Debug, Clone, Default)]
pub struct Clock(Arc<AtomicU64>);

impl Clock {
    pub fn next_frame(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    pub fn frame(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

type Task = Box<dyn FnOnce() + Send>;

/// As tarefas que as linhas de fundo mandaram para a linha da janela.
#[derive(Clone, Default)]
pub struct Queue(Arc<Mutex<VecDeque<Task>>>);

impl Queue {
    /// Roda o que chegou e diz quantas eram.
    pub fn run_all(&self) -> usize {
        let tasks: Vec<Task> = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain(..)
            .collect();
        let count = tasks.len();
        tasks.into_iter().for_each(|task| task());
        count
    }
}

impl EventLoopProxy for Queue {
    fn quit_event_loop(&self) -> Result<(), EventLoopError> {
        Ok(())
    }

    fn invoke_from_event_loop(&self, event: Task) -> Result<(), EventLoopError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back(event);
        Ok(())
    }
}

struct FilmPlatform {
    clock: Clock,
    queue: Queue,
}

impl Platform for FilmPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(headless::new_window())
    }

    fn duration_since_start(&self) -> Duration {
        frame_time(self.clock.frame())
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(self.queue.clone()))
    }
}

thread_local! {
    static INSTALLED: RefCell<Option<(Clock, Queue)>> = const { RefCell::new(None) };
}

/// Instala a plataforma; na segunda vez, devolve a que já está.
pub fn install() -> Result<(Clock, Queue), FilmError> {
    if let Some(installed) = INSTALLED.with_borrow(Clone::clone) {
        return Ok(installed);
    }
    let (clock, queue) = (Clock::default(), Queue::default());
    let platform = FilmPlatform {
        clock: clock.clone(),
        queue: queue.clone(),
    };
    slint::platform::set_platform(Box::new(platform))
        .map_err(|reason| FilmError::Window(reason.to_string()))?;
    INSTALLED.set(Some((clock.clone(), queue.clone())));
    Ok((clock, queue))
}

/// A janela, o relógio e a fila.
pub struct Stage {
    pub screen: Screen,
    pub clock: Clock,
    pub queue: Queue,
}

impl Stage {
    /// O relógio anda um quadro; o que chegou roda antes de `act`, a entrada desse quadro, e tudo
    /// antes do desenho (FI-B7).
    pub fn frame(&self, act: impl FnOnce()) -> Vec<Rgb8Pixel> {
        self.clock.next_frame();
        self.queue.run_all();
        act();
        slint::platform::update_timers_and_animations();
        self.queue.run_all();
        self.screen.paint()
    }
}
