//! As vagas da lâmina grande na tela (spec MV, regra 4; ADR 0011, decisão 2): a lista da tela tem
//! uma linha por vaga, e uma mudança chega por `set_row_data`, `push` ou `remove`, nunca trocando a
//! lista inteira (a não ser em outra lâmina ou outro álbum). Assim a mesma instância da tela
//! continua mostrando a mesma foto. Quem decide as vagas é o `Slots` do editor.

use std::rc::Rc;

use lamina_editor::FrameView;
use lamina_editor::motion::{Place, Slot, Slots};
use slint::{Model, ModelRc, VecModel};

use crate::FrameData;

pub struct SlotRows {
    slots: Slots,
    model: Rc<VecModel<FrameData>>,
}

impl SlotRows {
    pub fn new() -> Self {
        SlotRows {
            slots: Slots::default(),
            model: Rc::new(VecModel::default()),
        }
    }

    /// A lista da tela (sempre a mesma).
    pub fn model(&self) -> ModelRc<FrameData> {
        ModelRc::from(self.model.clone())
    }

    /// Leva a lista às vagas dos quadros `frames`; `rows[k]` é o desenho do quadro `k`. Devolve
    /// `true` se alguma vaga está esmaecendo: [`SlotRows::sweep`] a retira depois.
    pub fn sync(
        &mut self,
        frames: &[FrameView],
        rows: &[FrameData],
        place: Place,
        gesture: bool,
    ) -> bool {
        let reset = self.slots.update(frames, place, gesture);
        if reset {
            let all = self.slots.slots().iter().map(|s| row(s, rows, None));
            self.model.set_vec(all.collect::<Vec<_>>());
            return false;
        }
        let known = self.model.row_count();
        for (i, s) in self.slots.slots().iter().enumerate() {
            if i < known {
                let old = self.model.row_data(i);
                let new = row(s, rows, old.clone());
                if old.as_ref() != Some(&new) {
                    self.model.set_row_data(i, new);
                }
            } else {
                self.model.push(row(s, rows, None));
            }
        }
        self.slots.slots().iter().any(|s| !s.live)
    }

    /// As linhas mostradas agora (a lâmina que sai, ao navegar).
    pub fn rows(&self) -> Vec<FrameData> {
        self.model.iter().collect()
    }

    /// Sem lâmina para mostrar (álbum sem lâminas): a lista da tela fica vazia.
    pub fn clear(&mut self) {
        self.slots = Slots::default();
        self.model.set_vec(Vec::new());
    }

    /// Retira as vagas que terminaram de esmaecer (MV-B12), só por remoção.
    pub fn sweep(&mut self) {
        for i in self.slots.sweep() {
            self.model.remove(i);
        }
    }
}

/// A linha da vaga: o desenho do quadro dela, ou o último desenho, apagando-se, se ela saiu.
fn row(slot: &Slot, rows: &[FrameData], old: Option<FrameData>) -> FrameData {
    let id = i32::try_from(slot.id).unwrap_or(i32::MAX);
    match slot.frame.and_then(|k| rows.get(k)) {
        Some(data) => FrameData {
            id,
            fly: slot.fly,
            born: slot.born,
            live: true,
            ..data.clone()
        },
        None => FrameData {
            id,
            index: -1,
            fly: false,
            born: false,
            live: false,
            ..old.unwrap_or_default()
        },
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
    use slint::platform::{Platform, WindowAdapter};
    use slint::{
        Color, ComponentHandle, Image, Model, ModelRc, PhysicalSize, Rgb8Pixel, Rgba8Pixel,
        SharedPixelBuffer, VecModel,
    };

    use crate::{AppWindow, BoxData, FrameData, SpreadData};

    const WIDTH: u32 = 1600;
    const HEIGHT: u32 = 1000;
    /// A cor do quadro sem foto (`Fixed.placeholder`).
    const GREY: (u8, u8, u8) = (128, 128, 128);
    /// Mais que isto, é uma área: alguns poucos pixels dessa cor saem da borda das letras.
    const LOTS: usize = 1000;

    /// A janela de verdade, sem tela: desenha num buffer e deixa o teste escolher quando desenhar.
    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    fn draw(window: &MinimalSoftwareWindow) -> Vec<Rgb8Pixel> {
        let mut pixels = vec![Rgb8Pixel::default(); (WIDTH * HEIGHT) as usize];
        slint::platform::update_timers_and_animations();
        window.draw_if_needed(|r| {
            r.render(&mut pixels, WIDTH as usize);
        });
        pixels
    }

    fn count(pixels: &[Rgb8Pixel], (r, g, b): (u8, u8, u8)) -> usize {
        pixels
            .iter()
            .filter(|p| (p.r, p.g, p.b) == (r, g, b))
            .count()
    }

    fn red_image() -> Image {
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(4, 2);
        buffer.make_mut_slice().fill(Rgba8Pixel {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        });
        Image::from_rgba8(buffer)
    }

    /// Uma vaga que cobre a lâmina inteira, ainda sem foto ou com ela.
    fn slot(img: Option<Image>) -> FrameData {
        FrameData {
            id: 1,
            index: 0,
            kind: 1,
            live: true,
            w: 1.0,
            h: 0.5,
            pw: 1.0,
            ph: 0.5,
            op: 1.0,
            photo: 4,
            has_img: img.is_some(),
            img: img.unwrap_or_default(),
            ..FrameData::default()
        }
    }

    /// Como `refresh_spread` monta: cada desenho da lâmina traz uma lista nova de páginas inativas.
    fn spread(frames: &Rc<VecModel<FrameData>>) -> SpreadData {
        SpreadData {
            aspect: 0.5,
            bg: Color::from_rgb_u8(255, 255, 255),
            bg_right: Color::from_rgb_u8(255, 255, 255),
            // Sem o corte, a lâmina inteira ficaria sob o véu de fora do corte.
            trim: BoxData {
                w: 1.0,
                h: 0.5,
                ..BoxData::default()
            },
            inactive: ModelRc::from(Rc::new(VecModel::<BoxData>::default())),
            frames: ModelRc::from(frames.clone()),
            ..SpreadData::default()
        }
    }

    #[test]
    fn rl_b30_a_foto_que_chega_entre_dois_desenhos_aparece_na_vaga() {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let w = AppWindow::new().unwrap();
        window.set_size(PhysicalSize::new(WIDTH, HEIGHT));
        w.show().unwrap();
        w.set_has_project(true);
        w.set_spreads(ModelRc::from(Rc::new(VecModel::from(vec![
            SpreadData::default(),
        ]))));
        // Como `refresh_spread`: nenhum quadro em recorte (o véu do recorte mudaria as cores).
        w.set_crop_frame(-1);
        w.set_selected_frame(-1);
        // Sem animar: a lâmina fica logo com o tamanho final.
        w.global::<crate::Motion>().set_on(false);
        let rows = Rc::new(VecModel::from(vec![slot(None)]));
        w.set_slots(ModelRc::from(rows.clone()));
        w.set_current(spread(&rows));
        let before = draw(&window);
        assert!(count(&before, GREY) > LOTS, "a vaga sem foto aparece cinza");

        // Duas atualizações da lâmina antes do próximo desenho: a miniatura chega na segunda.
        w.set_current(spread(&rows));
        rows.set_row_data(0, slot(Some(red_image())));
        w.set_current(spread(&rows));
        let after = draw(&window);

        assert!(count(&after, GREY) < LOTS, "a vaga ficou cinza");
        assert!(count(&after, (255, 0, 0)) > LOTS, "a foto não entrou");
    }
}
