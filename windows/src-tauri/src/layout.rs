use serde::Serialize;

/// Tamanho fixo da janela: cabe a Island Expanded; a forma anima dentro dela (ADR 0003).
pub const WINDOW_SIZE: (f64, f64) = (500.0, 340.0);

/// Folga ao redor da Hit Region pra o hover não "piscar" na borda.
const HIT_PADDING: f64 = 6.0;

#[derive(Clone, Copy, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notch {
    pub width: f64,
    pub height: f64,
    pub has_notch: bool,
}

/// Retângulo em pixels físicos, origem no canto superior esquerdo do desktop virtual (como o Windows reporta).
#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Default)]
pub struct Layout {
    pub screen: Rect,
    /// Fator de escala (DPI) da tela alvo: pixels físicos por ponto lógico.
    pub scale: f64,
    pub notch: Notch,
    /// Tamanho atual da Island em px do frontend (definido por ele a cada troca de estado), antes do `ui_scale`.
    pub island: (f64, f64),
    /// Opção "Tamanho da Island": a janela e o zoom do webview crescem juntos por esse fator.
    pub ui_scale: f64,
}

impl Layout {
    /// O ponto (pixels físicos, desktop virtual) está dentro da Hit Region?
    pub fn hit(&self, x: f64, y: f64) -> bool {
        let s = self.screen;
        let ui = if self.ui_scale > 0.0 { self.ui_scale } else { 1.0 };
        let (w, h) = (self.island.0 * ui, self.island.1 * ui);
        if w <= 0.0 || h <= 0.0 || self.scale <= 0.0 {
            return false;
        }
        let lx = (x - s.x) / self.scale;
        let ly = (y - s.y) / self.scale;
        let cx = s.w / self.scale / 2.0;
        lx >= cx - w / 2.0 - HIT_PADDING
            && lx <= cx + w / 2.0 + HIT_PADDING
            && ly >= -1.0
            && ly <= h + HIT_PADDING
    }
}
