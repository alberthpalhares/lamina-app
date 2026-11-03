//! O plano do rolo de lâminas (spec RL, regras 5 a 9 e 25): a ordem dos pedidos, as revisões, o
//! que fazer com cada imagem que chega, a memória contada, as texturas, nunca em branco, a imagem
//! incompleta, redimensionar, outro álbum e a velocidade (RL-B2 a RL-B7, B9, B20, B22 e B26).

use std::time::{Duration, Instant};

use lamina_core::testing::{TRIES, budget};
use lamina_core::{Project, SpreadId};

use super::tl_common::{H, V, built, editor_at, single_start, spec};
use crate::motion::{
    Revisions, RollAsk, RollGot, RollPlan, RollView, SMALL_WIDTH, Tier, strip_rows, textured,
};
use crate::screens::Arrival;
use crate::{Command, Editor, Session};

/// Álbum de `spreads` lâminas com 4 fotos cada.
fn album(spreads: usize) -> Project {
    let photos: Vec<(u32, u32)> = (0..spreads * 4)
        .map(|i| if i % 4 == 3 { V } else { H })
        .collect();
    built(spec(30.0, 30.0), &photos, spreads)
}

fn revisions(session: &Session) -> Revisions {
    let mut revs = Revisions::default();
    revs.update(&strip_rows(session), &session.project().spec);
    revs
}

/// O que o rolo mostra, com as lâminas da tela (contando de 1).
fn view(center: usize, visible: (usize, usize), current: usize) -> RollView {
    RollView {
        center: center - 1,
        visible: visible.0 - 1..=visible.1 - 1,
        current: current - 1,
        current_idle: false,
        width: 1600,
    }
}

/// Os pedidos como (nível, lâmina da tela).
fn asked(asks: &[RollAsk]) -> Vec<(Tier, usize)> {
    asks.iter().map(|a| (a.tier, a.index + 1)).collect()
}

fn got(ask: RollAsk) -> RollGot {
    let height = ask.width / 2;
    RollGot {
        ask,
        complete: true,
        bytes: (ask.width * height * 4) as usize,
    }
}

/// O pedido de nível `tier` da lâmina `n` (da tela) na revisão de agora.
fn ask_for(plan: &RollPlan, revs: &Revisions, n: usize, tier: Tier) -> RollAsk {
    let spread = revs.order()[n - 1];
    RollAsk {
        epoch: plan.epoch(),
        spread,
        index: n - 1,
        tier,
        revision: revs.of(spread).unwrap(),
        width: if tier == Tier::Small {
            SMALL_WIDTH
        } else {
            1600
        },
    }
}

#[test]
fn rl_b2_pedidos_pequenos_primeiro_e_a_atual_so_depois_de_300_ms() {
    let editor = editor_at(album(20), 9);
    let revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    let mut v = view(10, (9, 11), 10);
    let first = asked(&plan.go(&v, &revs));
    use Tier::{Screen, Small};
    let expected = vec![
        (Small, 9),
        (Small, 11),
        (Small, 8),
        (Small, 12),
        (Screen, 9),
        (Screen, 11),
        (Screen, 8),
        (Screen, 12),
    ];
    assert_eq!(first, expected);
    // A lâmina 10 ficou 300 ms sem edição: a imagem de tela dela vem no fim da fila.
    v.current_idle = true;
    let mut later = expected;
    later.push((Screen, 10));
    assert_eq!(asked(&plan.go(&v, &revs)), later);
}

#[test]
fn rl_b3_so_sobe_a_revisao_do_que_a_imagem_mostra() {
    let project = built(
        single_start(spec(30.0, 30.0)),
        &[H, H, V, H, H, V, H, H, V, H, H, V, H, H, V, H],
        8,
    );
    let mut editor = editor_at(project, 2);
    let mut revs = revisions(editor.session().unwrap());
    let ids: Vec<SpreadId> = revs.order().to_vec();
    let mut update = |editor: &Editor| {
        let s = editor.session().unwrap();
        revs.update(&strip_rows(s), &s.project().spec)
    };
    // Trocar o layout da lâmina 3.
    assert!(editor.run(Command::NextLayout));
    assert_eq!(update(&editor), vec![ids[2]]);
    // Mover a lâmina 5 para a posição 6: nada.
    editor.session_mut().unwrap().go_to(4);
    assert!(editor.move_current_to(5));
    assert!(update(&editor).is_empty());
    // O corte entre a 6 e a 7: as duas.
    assert!(editor.drag_cut(5, 1));
    let order = editor.session().unwrap().project().spreads.clone();
    let mut cut = update(&editor);
    cut.sort();
    let mut both = vec![order[5].id, order[6].id];
    both.sort();
    assert_eq!(cut, both);
    // A lâmina 2 para o início: ela e a antiga primeira (as páginas ativas mudaram).
    let old_first = order[0].id;
    editor.session_mut().unwrap().go_to(1);
    assert!(editor.move_current_to(0));
    let mut moved = update(&editor);
    moved.sort();
    let mut edges = vec![order[1].id, old_first];
    edges.sort();
    assert_eq!(moved, edges);
    // Só o aviso mudou: nada.
    let s = editor.session().unwrap();
    let mut rows = strip_rows(s);
    rows[3].1.warned = !rows[3].1.warned;
    assert!(revs.update(&rows, &s.project().spec).is_empty());
}

#[test]
fn rl_b4_imagem_velha_pior_ou_fora_do_alcance_e_descartada() {
    let mut editor = editor_at(album(60), 5);
    let mut revs = revisions(editor.session().unwrap());
    // A lâmina 6 vai à revisão 3 (duas trocas de layout).
    for _ in 0..2 {
        assert!(editor.run(Command::NextLayout));
        let s = editor.session().unwrap();
        revs.update(&strip_rows(s), &s.project().spec);
    }
    let six = revs.order()[5];
    assert_eq!(revs.of(six), Some(3));
    let mut plan = RollPlan::default();
    plan.go(&view(5, (4, 7), 5), &revs);
    let screen = ask_for(&plan, &revs, 6, Tier::Screen);
    let old = RollAsk {
        revision: 2,
        ..screen
    };
    assert_eq!(plan.arrived(got(old), false).arrival, Arrival::Discard);
    assert_eq!(plan.arrived(got(screen), false).arrival, Arrival::Show);
    let small = ask_for(&plan, &revs, 6, Tier::Small);
    assert_eq!(plan.arrived(got(small), false).arrival, Arrival::Discard);
    let far = ask_for(&plan, &revs, 40, Tier::Screen);
    assert_eq!(plan.arrived(got(far), false).arrival, Arrival::Discard);
    // Durante um deslize, a imagem nova é segurada e entra quando ele termina.
    let mut sliding = RollPlan::default();
    sliding.go(&view(5, (4, 7), 5), &revs);
    let landing = sliding.arrived(got(screen), true);
    assert_eq!(landing.arrival, Arrival::Hold);
    assert_eq!(sliding.slide_ended(), vec![six]);
    assert!(sliding.sharp(six));
}

#[test]
fn rl_b5_memoria_contada_tira_a_mais_longe_e_no_empate_a_de_maior_indice() {
    let editor = editor_at(album(30), 9);
    let revs = revisions(editor.session().unwrap());
    let one = got(ask_for(&RollPlan::default(), &revs, 6, Tier::Screen)).bytes;
    let run = || {
        let mut plan = RollPlan::with_cap(one * 5);
        plan.go(&view(10, (8, 12), 10), &revs);
        let mut out = Vec::new();
        for n in 6..=14 {
            let landing = plan.arrived(got(ask_for(&plan, &revs, n, Tier::Screen)), false);
            out.extend(landing.evict);
        }
        (plan, out)
    };
    let (plan, out) = run();
    let ids = revs.order();
    assert_eq!(out, vec![ids[5], ids[6], ids[12], ids[13]]);
    for n in 8..=12 {
        assert!(plan.kept(ids[n - 1]).is_some(), "a lâmina {n} fica");
    }
    assert_eq!(run().1, out, "a mesma sequência dá o mesmo resultado");
    // Empate: a 6 e a 14 estão a 4 lâminas do centro; sai a 14.
    let mut tie = RollPlan::with_cap(one * 2);
    tie.go(&view(10, (8, 12), 10), &revs);
    tie.arrived(got(ask_for(&tie, &revs, 6, Tier::Screen)), false);
    let landing = tie.arrived(got(ask_for(&tie, &revs, 14, Tier::Screen)), false);
    assert!(landing.evict.is_empty());
    let landing = tie.arrived(got(ask_for(&tie, &revs, 10, Tier::Screen)), false);
    assert_eq!(landing.evict, vec![ids[13]]);
}

#[test]
fn rl_b6_so_as_visiveis_mais_uma_de_cada_lado_seguram_textura() {
    let editor = editor_at(album(75), 9);
    let revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    plan.go(&view(10, (5, 15), 10), &revs);
    for n in 5..=15 {
        plan.arrived(got(ask_for(&plan, &revs, n, Tier::Screen)), false);
    }
    let ids = revs.order();
    // O modelo da janela: linhas à vista ±1 que já têm imagem.
    let rows = |plan: &RollPlan, visible: std::ops::RangeInclusive<usize>| {
        textured(&visible, 75)
            .filter(|i| plan.kept(ids[*i]).is_some())
            .map(|i| i + 1)
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(&plan, 8..=10), vec![8, 9, 10, 11, 12]);
    // Rolou até a 30: nenhuma das de antes segura textura; as novas entram ao ficarem prontas.
    plan.go(&view(30, (29, 31), 10), &revs);
    assert!(rows(&plan, 28..=30).is_empty());
    plan.arrived(got(ask_for(&plan, &revs, 29, Tier::Small)), false);
    assert_eq!(rows(&plan, 28..=30), vec![29]);
    assert!(
        plan.kept(ids[9]).is_some(),
        "a 10 continua guardada no Rust"
    );
}

#[test]
fn rl_b7_nunca_em_branco_a_imagem_velha_fica_ate_a_nova_chegar() {
    let mut editor = editor_at(album(12), 4);
    let mut revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    plan.go(&view(5, (4, 8), 5), &revs);
    let eight = revs.order()[7];
    plan.arrived(got(ask_for(&plan, &revs, 8, Tier::Screen)), false);
    // A 7 não tem imagem: a janela mostra o número; a 8 tem a da revisão 1.
    assert!(plan.kept(revs.order()[6]).is_none());
    editor.session_mut().unwrap().go_to(7);
    assert!(editor.run(Command::NextLayout));
    let s = editor.session().unwrap();
    revs.update(&strip_rows(s), &s.project().spec);
    plan.go(&view(5, (4, 8), 8), &revs);
    let kept = plan.kept(eight).unwrap();
    assert_eq!(
        kept.ask.revision, 1,
        "a imagem velha continua até a nova chegar"
    );
    assert!(!plan.sharp(eight));
}

#[test]
fn rl_b9_imagem_incompleta_e_pedida_de_novo_quando_o_cache_muda() {
    let editor = editor_at(album(10), 0);
    let revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    let v = view(1, (1, 2), 1);
    plan.go(&v, &revs);
    let ask = ask_for(&plan, &revs, 2, Tier::Screen);
    let partial = RollGot {
        complete: false,
        ..got(ask)
    };
    assert_eq!(plan.arrived(partial, false).arrival, Arrival::Show);
    let screens = |plan: &mut RollPlan| {
        asked(&plan.go(&v, &revs))
            .into_iter()
            .filter(|(tier, n)| *tier == Tier::Screen && *n == 2)
            .count()
    };
    assert_eq!(screens(&mut plan), 0, "sem o cache mudar, não pede de novo");
    plan.recheck();
    assert_eq!(screens(&mut plan), 1);
    let again = plan.go(&v, &revs);
    assert!(again.contains(&ask), "na mesma revisão");
}

#[test]
fn rl_b20_redimensionar_pede_de_novo_so_acima_de_10_por_cento() {
    let editor = editor_at(album(10), 4);
    let revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    let mut v = view(5, (4, 6), 5);
    plan.go(&v, &revs);
    for n in [3, 4, 6, 7] {
        plan.arrived(got(ask_for(&plan, &revs, n, Tier::Screen)), false);
    }
    let screens = |asks: Vec<RollAsk>| asks.iter().filter(|a| a.tier == Tier::Screen).count();
    v.width = 1900;
    let asks = plan.go(&v, &revs);
    assert!(asks.iter().all(|a| a.width == 1900));
    assert_eq!(screens(asks), 4);
    v.width = 1650;
    assert_eq!(screens(plan.go(&v, &revs)), 0, "menos de 10%: nada");
}

#[test]
fn rl_b22_outro_album_descarta_pedidos_e_imagens_e_revisoes_recomecam() {
    let editor = editor_at(album(10), 4);
    let mut revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    let v = view(5, (4, 6), 5);
    plan.go(&v, &revs);
    let pending = ask_for(&plan, &revs, 6, Tier::Screen);
    plan.arrived(got(ask_for(&plan, &revs, 4, Tier::Screen)), false);
    // O álbum B, com os mesmos ids de lâmina.
    let other = editor_at(album(10), 0);
    revs.clear();
    plan.new_album();
    let s = other.session().unwrap();
    revs.update(&strip_rows(s), &s.project().spec);
    assert!(revs.order().iter().all(|id| revs.of(*id) == Some(1)));
    plan.go(&v, &revs);
    assert!(plan.kept(revs.order()[3]).is_none());
    assert_eq!(plan.arrived(got(pending), false).arrival, Arrival::Discard);
}

#[test]
fn rl_b26_revisoes_e_plano_rapido() {
    // A montagem automática põe no máximo 8 fotos por lâmina: 1.600 nas lâminas e 400 só no álbum.
    let photos: Vec<(u32, u32)> = (0..1600).map(|i| if i % 4 == 3 { V } else { H }).collect();
    let mut project = built(spec(30.0, 30.0), &photos, 200);
    for i in 0..400 {
        project.add_photo(format!("extra{i:03}.jpg"), H.0, H.1);
    }
    let mut editor = editor_at(project, 100);
    let mut revs = revisions(editor.session().unwrap());
    let mut plan = RollPlan::default();
    let v = view(101, (100, 102), 101);
    let mut best = Duration::MAX;
    for _ in 0..TRIES {
        assert!(editor.run(Command::NextLayout));
        // As linhas da faixa são as mesmas que a janela já calcula para a faixa de lâminas.
        let s = editor.session().unwrap();
        let rows = strip_rows(s);
        let started = Instant::now();
        let changed = revs.update(&rows, &s.project().spec);
        let asks = plan.go(&v, &revs);
        best = best.min(started.elapsed());
        assert_eq!(changed.len(), 1);
        assert!(!asks.is_empty());
        if best < budget(5) {
            break;
        }
    }
    assert!(best < budget(5), "revisões e plano levaram {best:?}");
}
