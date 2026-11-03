//! Tamanho do texto da interface (spec DF, regra DF-B14): um fator que multiplica a escala de
//! letras da janela (`Type` em `ui/theme/type.slint`). Fica entre 0,9 e 1,3, para a janela caber
//! num notebook e nada sair do lugar; quem escolhe é a preferência (spec AJ, regra 15).

/// O menor e o maior fator aceitos.
pub const TEXT_SCALE_RANGE: (f32, f32) = (0.9, 1.3);

/// O fator pedido, limitado ao intervalo; um valor inválido (não é número) vira 1,0.
pub fn text_scale(requested: f32) -> f32 {
    if requested.is_nan() {
        return 1.0;
    }
    requested.clamp(TEXT_SCALE_RANGE.0, TEXT_SCALE_RANGE.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn df_b14_escala_de_texto_limitada() {
        assert_eq!(text_scale(0.5), 0.9);
        assert_eq!(text_scale(2.0), 1.3);
        assert_eq!(text_scale(1.1), 1.1);
    }

    #[test]
    fn escala_invalida_volta_ao_normal() {
        assert_eq!(text_scale(f32::NAN), 1.0);
    }
}
