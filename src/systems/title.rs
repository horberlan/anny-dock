use bevy::prelude::*;
use crate::types::*;


pub fn toggle_titles(
    mut show_titles: ResMut<ShowTitles>,
    keyboard_input: Res<Input<KeyCode>>,
    mut q_texts: Query<&mut Visibility, With<IconTitleText>>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        show_titles.0 = !show_titles.0;

        let new_visibility = if show_titles.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        for mut visibility in q_texts.iter_mut() {
            *visibility = new_visibility;
        }
    }
}
