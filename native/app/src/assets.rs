// Copyright (c) 2026 Tymofii Kosovskyi
// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

//! The app's assets: the Tauri build's icon set first (see
//! `icons/extract_icons.py`), then everything GPUI Kit ships.

use std::borrow::Cow;

use gpui_kit::component::IconNamed;
use gpui_kit::{AssetSource, SharedString};

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

impl IconNamed for AppIcon {
    fn path(self) -> SharedString {
        self.asset_path().into()
    }
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Some(icon) = AppIcon::ALL.iter().find(|icon| icon.asset_path() == path) {
            return Ok(Some(Cow::Borrowed(icon.svg())));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        let mut found: Vec<SharedString> =
            AppIcon::ALL.iter().map(|icon| icon.asset_path()).filter(|p| p.starts_with(path)).map(Into::into).collect();
        found.extend(gpui_kit::assets::Assets.list(path)?);
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: that would bring GPUI's `test` attribute over the built-in one.
    use super::{AppIcon, Assets};
    use gpui_kit::AssetSource as _;

    #[test]
    fn every_icon_is_served_at_its_own_path_and_is_an_svg() {
        let mut paths: Vec<&str> = AppIcon::ALL.iter().map(|i| i.asset_path()).collect();
        paths.sort_unstable();
        paths.dedup();
        assert_eq!(paths.len(), AppIcon::ALL.len(), "one path per icon");
        for icon in AppIcon::ALL {
            let served = Assets.load(icon.asset_path()).unwrap().expect("served");
            let text = std::str::from_utf8(&served).unwrap();
            assert!(text.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\"") && text.trim_end().ends_with("</svg>"), "{icon:?}");
        }
        // The kit's own icons are still there behind ours.
        assert!(Assets.list("icons/").unwrap().len() > AppIcon::ALL.len());
    }
}
