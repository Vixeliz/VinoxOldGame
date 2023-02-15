use bevy::prelude::*;

pub fn start(mut windows: ResMut<Windows>) {
    let windows = windows.get_primary_mut().unwrap();
    windows.set_cursor_icon(CursorIcon::Crosshair);
}

pub fn despawn_with<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in q.iter() {
        commands.entity(e).despawn_recursive();
    }
}
