use serde::Serialize;

/// Tamanho fixo da janela: cabe a Island Expanded; a forma anima dentro dela (ADR 0003).
pub const WINDOW_SIZE: (f64, f64) = (500.0, 260.0);

/// Folga ao redor da Hit Region pra o hover não "piscar" na borda.
const HIT_PADDING: f64 = 6.0;

#[derive(Clone, Copy, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notch {
    pub width: f64,
    pub height: f64,
    pub has_notch: bool,
}

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
    pub notch: Notch,
    /// Tamanho atual da Island (definido pelo frontend a cada troca de estado).
    pub island: (f64, f64),
}

impl Layout {
    /// Ponto em coordenadas Cocoa está dentro da Hit Region?
    pub fn hit(&self, x: f64, y: f64) -> bool {
        let s = self.screen;
        let (w, h) = self.island;
        let cx = s.x + s.w / 2.0;
        let top = s.y + s.h;
        x >= cx - w / 2.0 - HIT_PADDING
            && x <= cx + w / 2.0 + HIT_PADDING
            && y >= top - h - HIT_PADDING
            && y <= top + 1.0
    }
}
