//! A janela de verdade sem tela: a `AppWindow` desenha num buffer, e quem chama escolhe quando (spec
//! RL, regra 33). Serve aos testes e à filmagem da interface (spec FI). Cada chamada abre uma janela
//! nova, e a plataforma já instalada é aceita, para rodar também com `--test-threads=1`.

use std::cell::RefCell;
use std::rc::Rc;

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, PlatformError, Rgb8Pixel};

use crate::AppWindow;

thread_local! {
    /// A última janela criada pela plataforma nesta linha de execução.
    static LATEST: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

struct Headless;

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        LATEST.set(Some(window.clone()));
        Ok(window)
    }
}

/// Instala a plataforma sem janela do sistema. O único erro possível é `AlreadySet`: já instalada,
/// segue com ela.
pub fn install() {
    let _ = slint::platform::set_platform(Box::new(Headless));
}

/// A `AppWindow` aberta sem tela, com o tamanho dado em pixels.
pub struct Screen {
    pub app: AppWindow,
    pub window: Rc<MinimalSoftwareWindow>,
    width: u32,
    height: u32,
}

/// Abre uma `AppWindow` nova com `width`×`height`.
pub fn open(width: u32, height: u32) -> Result<Screen, PlatformError> {
    install();
    let app = AppWindow::new()?;
    // A plataforma só cria a janela quando alguém pede.
    let _ = app.window();
    let window = LATEST
        .take()
        .ok_or_else(|| PlatformError::Other("a plataforma não criou a janela".to_owned()))?;
    window.set_size(PhysicalSize::new(width, height));
    app.show()?;
    Ok(Screen {
        app,
        window,
        width,
        height,
    })
}

impl Screen {
    /// Roda os temporizadores e desenha a janela inteira num buffer novo.
    pub fn draw(&self) -> Vec<Rgb8Pixel> {
        let (width, height) = (self.width as usize, self.height as usize);
        let mut pixels = vec![Rgb8Pixel::default(); width * height];
        slint::platform::update_timers_and_animations();
        self.window.request_redraw();
        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width);
        });
        pixels
    }

    /// Troca o tamanho da janela.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window.set_size(PhysicalSize::new(width, height));
        (self.width, self.height) = (width, height);
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
