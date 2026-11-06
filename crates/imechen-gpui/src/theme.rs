use gpui::{rgb, Hsla, WindowAppearance};

#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: Hsla,
    pub surface: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub accent: Hsla,
    pub accent_text: Hsla,
    pub disabled: Hsla,
    pub danger: Hsla,
    pub success: Hsla,
    pub drop_active: Hsla,
}

impl Palette {
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }

    fn light() -> Self {
        Self {
            bg: rgb(0xf4f5f7).into(),
            surface: rgb(0xffffff).into(),
            border: rgb(0xd5d8de).into(),
            text: rgb(0x1c1e22).into(),
            text_muted: rgb(0x6b7280).into(),
            accent: rgb(0x2563eb).into(),
            accent_text: rgb(0xffffff).into(),
            disabled: rgb(0xb8bdc7).into(),
            danger: rgb(0xdc2626).into(),
            success: rgb(0x15803d).into(),
            drop_active: rgb(0xdbe7ff).into(),
        }
    }

    fn dark() -> Self {
        Self {
            bg: rgb(0x16181d).into(),
            surface: rgb(0x23262d).into(),
            border: rgb(0x3a3f4a).into(),
            text: rgb(0xe8eaee).into(),
            text_muted: rgb(0x9aa1ad).into(),
            accent: rgb(0x3b82f6).into(),
            accent_text: rgb(0xffffff).into(),
            disabled: rgb(0x4a4f5a).into(),
            danger: rgb(0xf87171).into(),
            success: rgb(0x4ade80).into(),
            drop_active: rgb(0x1e3a6b).into(),
        }
    }
}
