use crate::networking::components::Player;
use bevy::{gltf::Gltf, math::Vec2Swizzles, prelude::*, render::primitives::Aabb};
use bevy_rapier3d::{prelude::*, rapier::prelude::InteractionGroups};

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
    pub collision_groups: CollisionGroups,
}

#[derive(Resource, Default)]
pub struct PlayerBundleBuilder {
    pub default_model: Handle<Scene>,
    pub local_model: Handle<Scene>,
    pub model_aabb: Aabb,
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
    pub fn build(&self, translation: Vec3, id: u64, local: bool) -> PlayerBundle {
        let handle = if local {
            self.local_model.clone()
        } else {
            self.default_model.clone()
        };

        let visibility = if local {
            Visibility::INVISIBLE
        } else {
            Visibility::VISIBLE
        };
        PlayerBundle {
            collider: ColliderBundle {
                collider: Collider::cuboid(
                    self.model_aabb.half_extents.x,
                    self.model_aabb.half_extents.y,
                    self.model_aabb.half_extents.z,
                ),
                rigid_body: RigidBody::KinematicVelocityBased,
                rotation_constraints: LockedAxes::ROTATION_LOCKED,
                collision_groups: CollisionGroups::new(
                    Group::GROUP_1,
                    Group::from_bits_truncate(Group::GROUP_2.bits()),
                ),
                ..Default::default()
            },
            player_tag: Player { id },
            scene_bundle: SceneBundle {
                scene: handle,
                transform: Transform::from_translation(translation),
                visibility,
                ..default()
            },
            ..Default::default()
        }
    }
}
