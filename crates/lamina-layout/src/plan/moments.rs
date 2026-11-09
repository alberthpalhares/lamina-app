//! Regras 1 a 3 da spec: ordem das fotos, corte em momentos e união de momentos vizinhos.

use lamina_core::{Photo, PhotoId};

/// Intervalo "infinito" antes do momento das fotos sem data: é o último a ser unido.
pub const NO_DATE_GAP: i64 = i64::MAX;

/// Fotos em ordem: por data de captura; empate e fotos sem data pelo nome do arquivo (regra 1).
/// As fotos na ordem de captura (data, depois nome). Com `order` não vazio (spec OF, beta 9.1),
/// a ordem do critério escolhido pelo painel.
pub fn ordered<'a>(photos: &'a [Photo], order: &[PhotoId]) -> Vec<&'a Photo> {
    let mut list: Vec<&Photo> = photos.iter().collect();
    if order.is_empty() {
        list.sort_by_key(|p| {
            let name = p.path.file_name().map(|n| n.to_string_lossy().into_owned());
            (p.taken_at.is_none(), p.taken_at, name, p.path.clone())
        });
    } else {
        let pos = |id: PhotoId| order.iter().position(|o| *o == id);
        list.sort_by_key(|p| (pos(p.id).is_none(), pos(p.id).unwrap_or(usize::MAX)));
    }
    list
}

/// Momentos: grupos de posições consecutivas em `ordered` e o intervalo entre cada par vizinho
/// (`gaps[i]` separa o momento `i` do `i + 1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moments {
    pub groups: Vec<Vec<usize>>,
    pub gaps: Vec<i64>,
}

impl Moments {
    /// Corta a sequência quando o intervalo passa de `max_gap` segundos (regra 2).
    pub fn split(ordered: &[&Photo], max_gap: i64) -> Moments {
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut gaps = Vec::new();
        for (i, photo) in ordered.iter().enumerate() {
            let gap = i
                .checked_sub(1)
                .and_then(|j| boundary_gap(ordered[j], photo, max_gap));
            match gap {
                Some(gap) => {
                    gaps.push(gap);
                    groups.push(vec![i]);
                }
                None => match groups.last_mut() {
                    Some(last) => last.push(i),
                    None => groups.push(vec![i]),
                },
            }
        }
        Moments { groups, gaps }
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn sizes(&self) -> Vec<usize> {
        self.groups.iter().map(Vec::len).collect()
    }

    /// Une o momento `i` ao `i + 1`.
    pub fn merge_with_next(&mut self, i: usize) {
        let next = self.groups.remove(i + 1);
        self.groups[i].extend(next);
        self.gaps.remove(i);
    }

    /// Une os vizinhos mais próximos até sobrarem no máximo `max` momentos (regra 3).
    pub fn merge_until(&mut self, max: usize) {
        while self.len() > max.max(1) {
            let closest = self
                .gaps
                .iter()
                .enumerate()
                .min_by_key(|(i, g)| (**g, *i))
                .map_or(0, |(i, _)| i);
            self.merge_with_next(closest);
        }
    }

    /// Une o momento `i` ao vizinho mais próximo (usado quando ele não cabe nas lâminas).
    pub fn merge_with_closest_neighbor(&mut self, i: usize) {
        let before = i.checked_sub(1).map(|j| self.gaps[j]);
        let after = self.gaps.get(i).copied();
        match (before, after) {
            (Some(b), Some(a)) if b <= a => self.merge_with_next(i - 1),
            (_, Some(_)) => self.merge_with_next(i),
            (Some(_), None) => self.merge_with_next(i - 1),
            (None, None) => {}
        }
    }
}

/// Intervalo que separa `prev` de `photo` quando um novo momento começa entre eles; `None` se
/// as duas ficam no mesmo momento.
fn boundary_gap(prev: &Photo, photo: &Photo, max_gap: i64) -> Option<i64> {
    match (prev.taken_at, photo.taken_at) {
        (Some(a), Some(b)) => {
            let gap = a.seconds_until(b);
            (gap > max_gap).then_some(gap)
        }
        (Some(_), None) => Some(NO_DATE_GAP),
        // Fotos sem data vêm todas no fim (regra 1) e formam um momento só.
        (None, _) => None,
    }
}
