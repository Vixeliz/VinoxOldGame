use crate::networking::components::Player;
use bevy::{math::Vec2Swizzles, prelude::*, gltf::Gltf};
use bevy_rapier3d::prelude::*;

#[derive(Resource, Default)]
pub struct AssetsLoading(pub Vec<HandleUntyped>);

#[derive(Clone, Debug, Default, Bundle)]
pub struct ColliderBundle {
    pub collider: Collider,
    pub rigid_body: RigidBody,
    pub velocity: Velocity,
    pub friction: Friction,
    pub density: ColliderMassProperties,
    pub rotation_constraints: LockedAxes,
}

#[derive(Resource, Default)]
pub struct PlayerBundleBuilder {
    pub default_model: Handle<Scene>,
}

#[derive(Default, Bundle)]
pub struct PlayerBundle {
    pub player_tag: Player,
    #[bundle]
    pub colliding_entities: CollidingEntities,
    #[bundle]
    pub collider: ColliderBundle,
    #[bundle]
    pub scene_bundle: SceneBundle, 
}

impl PlayerBundleBuilder {
    pub fn build(&self, translation: Vec3, id: u64) -> PlayerBundle {
        PlayerBundle {
            collider: ColliderBundle {
                collider: Collider::cuboid(1., 1., 1.),
                rigid_body: RigidBody::Dynamic,
                rotation_constraints: LockedAxes::ROTATION_LOCKED,
                ..Default::default()
            },
            player_tag: Player { id },
            scene_bundle: SceneBundle {
                scene: self.default_model.clone(),
                transform: Transform::from_translation(translation),
                ..default() 
            },
            ..Default::default()
        }
    }
}
