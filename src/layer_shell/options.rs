//! Translation of `LayerShellConfig` into layer-shell protocol values.

use smithay_client_toolkit::shell::wlr_layer::{KeyboardInteractivity, Layer};

use crate::config::{DockKeyboard, DockLayer};

pub fn layer(layer: DockLayer) -> Layer {
    match layer {
        DockLayer::Background => Layer::Background,
        DockLayer::Bottom => Layer::Bottom,
        DockLayer::Top => Layer::Top,
        DockLayer::Overlay => Layer::Overlay,
    }
}

pub fn keyboard(keyboard: DockKeyboard) -> KeyboardInteractivity {
    match keyboard {
        DockKeyboard::None => KeyboardInteractivity::None,
        DockKeyboard::Exclusive => KeyboardInteractivity::Exclusive,
        DockKeyboard::OnDemand => KeyboardInteractivity::OnDemand,
    }
}

/// The output named `wanted`, or `None` (compositor's choice) when it is empty.
pub fn pick_output<T>(outputs: Vec<(Option<String>, T)>, wanted: &str) -> Result<Option<T>, String> {
    if wanted.is_empty() {
        return Ok(None);
    }
    let names: Vec<String> = outputs.iter().filter_map(|(name, _)| name.clone()).collect();
    outputs
        .into_iter()
        .find(|(name, _)| name.as_deref() == Some(wanted))
        .map(|(_, output)| Some(output))
        .ok_or_else(|| format!("output {wanted:?} not found (available: {})", names.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_output_by_name() {
        let outputs = vec![(Some("eDP-1".to_string()), 1), (Some("HDMI-A-1".to_string()), 2)];

        assert_eq!(pick_output(outputs.clone(), "HDMI-A-1"), Ok(Some(2)));
        assert_eq!(pick_output(outputs.clone(), ""), Ok(None));
        assert_eq!(
            pick_output(outputs, "DP-3"),
            Err("output \"DP-3\" not found (available: eDP-1, HDMI-A-1)".to_string())
        );
    }

    #[test]
    fn maps_config_to_protocol_values() {
        assert_eq!(layer(DockLayer::Background), Layer::Background);
        assert_eq!(layer(DockLayer::Bottom), Layer::Bottom);
        assert_eq!(layer(DockLayer::Top), Layer::Top);
        assert_eq!(layer(DockLayer::Overlay), Layer::Overlay);
        assert_eq!(keyboard(DockKeyboard::None), KeyboardInteractivity::None);
        assert_eq!(keyboard(DockKeyboard::Exclusive), KeyboardInteractivity::Exclusive);
        assert_eq!(keyboard(DockKeyboard::OnDemand), KeyboardInteractivity::OnDemand);
    }
}
