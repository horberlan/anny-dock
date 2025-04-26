use bevy::prelude::*;
use crate::types::MainCamera;

pub fn cleanup_duplicate_cameras(mut commands: Commands, query: Query<(Entity, &MainCamera)>) {
    let mut found_camera = false;
    for (entity, _) in query.iter() {
        if found_camera {
            commands.entity(entity).despawn_recursive();
        } else {
            found_camera = true;
        }
    }
} 