//! Uma novidade do `lamina-editor` vira uma linha ou um cartão da página.

use lamina_editor::updates::news::{NewsEntry, NewsKind, TryNow, animates, video_address};

use super::covers::Covers;
use super::embedded;
use crate::{NewsCardData, NewsEntryData};

/// Os bytes da ilustração do item, que vai dentro do programa.
pub fn bytes_of(entry: &NewsEntry) -> Option<&'static [u8]> {
    let name = entry.item.image.as_deref()?;
    let (_, bytes) = embedded::IMAGES.iter().find(|(n, _)| *n == name)?;
    Some(bytes)
}

fn video_of(entry: &NewsEntry) -> &str {
    entry
        .item
        .video
        .as_deref()
        .and_then(video_address)
        .unwrap_or_default()
}

fn index_of(entry: &NewsEntry) -> i32 {
    i32::try_from(entry.index).unwrap_or(-1)
}

/// A linha da novidade. A capa já na memória entra direto; as outras a linha pede quando nasce.
pub fn row(
    entry: &NewsEntry,
    try_now: Option<&TryNow>,
    reduce_motion: bool,
    covers: &Covers,
) -> NewsEntryData {
    let item = &entry.item;
    let bytes = bytes_of(entry);
    let video = video_of(entry);
    let cover = covers.get(entry.index);
    let (command, ready, hint) = match try_now {
        Some(TryNow::Ready) => (item.command.as_deref().unwrap_or_default(), true, ""),
        Some(TryNow::Unavailable(hint)) => (
            item.command.as_deref().unwrap_or_default(),
            false,
            hint.as_str(),
        ),
        Some(TryNow::Hidden) | None => ("", false, ""),
    };
    NewsEntryData {
        index: index_of(entry),
        date: entry.date.as_str().into(),
        version: entry.version.as_str().into(),
        intro: entry.intro.as_str().into(),
        current: entry.current,
        label: entry.label.as_str().into(),
        text: item.text.as_str().into(),
        try_it: item.try_it.as_deref().unwrap_or_default().into(),
        command: command.into(),
        try_ready: ready,
        try_hint: hint.into(),
        video: video.into(),
        kind_tag: item.kind.map(NewsKind::label).unwrap_or_default().into(),
        kind_new: item.kind == Some(NewsKind::New),
        system_tag: item.only.map(|s| s.label()).unwrap_or_default().into(),
        has_file: bytes.is_some(),
        // Com "Reduzir movimento", a capa de animação fica parada; o vídeo continua abrindo.
        animated: !reduce_motion && video.is_empty() && bytes.is_some_and(animates),
        has_cover: cover.is_some(),
        cover: cover.unwrap_or_default(),
    }
}

/// O cartão da faixa de destaques.
pub fn card(entry: &NewsEntry, covers: &Covers) -> NewsCardData {
    let cover = covers.get(entry.index);
    NewsCardData {
        index: index_of(entry),
        label: entry.label.as_str().into(),
        has_file: bytes_of(entry).is_some(),
        video: video_of(entry).into(),
        has_cover: cover.is_some(),
        cover: cover.unwrap_or_default(),
    }
}
