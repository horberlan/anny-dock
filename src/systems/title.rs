use bevy::prelude::*;
use crate::types::*;
use crate::components::add_icon_text;

pub fn toggle_titles(
    mut commands: Commands,
    mut show_titles: ResMut<ShowTitles>,
    keyboard_input: Res<Input<KeyCode>>,
    q_icons: Query<(Entity, &Transform, &ClientClass, &HoverTarget)>,
    asset_server: Res<AssetServer>,
    q_texts: Query<Entity, With<IconText>>,
) {
    if keyboard_input.just_pressed(KeyCode::T) {
        show_titles.0 = !show_titles.0;

        if show_titles.0 {
            for (entity, transform, class, _hover) in q_icons.iter() {
                add_icon_text(
                    &mut commands,
                    entity,
                    &class.0,
                    *transform,
                    transform.scale.y,
                    &asset_server,
                );
            }
        } else {
            for entity in q_texts.iter() {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}
