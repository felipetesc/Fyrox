//! Contains all structures and methods to create and manage scene graph nodes.
//!
//! For more info see [`Node`]

#![warn(missing_docs)]

use crate::scene::graph::GraphUpdateSwitches;
use crate::scene::navmesh::NavigationalMesh;
use crate::scene::Scene;
use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        self,
        base::Base,
        camera::Camera,
        decal::Decal,
        dim2::{self, rectangle::Rectangle},
        graph::{self, Graph, NodePool},
        light::{point::PointLight, spot::SpotLight},
        mesh::Mesh,
        particle_system::ParticleSystem,
        sound::{context::SoundContext, listener::Listener, Sound},
        sprite::Sprite,
        terrain::Terrain,
    },
};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub mod constructor;
pub mod container;

/// A trait for an entity that has unique type identifier.
pub trait TypeUuidProvider: Sized {
    /// Return type UUID.
    fn type_uuid() -> Uuid;
}

/// A set of useful methods that is possible to auto-implement.
pub trait BaseNodeTrait: Any + Debug + Deref<Target = Base> + DerefMut + Send {
    /// This method creates raw copy of a node, it should never be called in normal circumstances
    /// because internally nodes may (and most likely will) contain handles to other nodes. To
    /// correctly clone a node you have to use [copy_node](struct.Graph.html#method.copy_node).
    fn clone_box(&self) -> Node;

    /// Casts self as `Any`
    fn as_any_ref(&self) -> &dyn Any;

    /// Casts self as `Any`
    fn as_any_ref_mut(&mut self) -> &mut dyn Any;
}

impl<T> BaseNodeTrait for T
where
    T: Clone + NodeTrait + 'static,
{
    fn clone_box(&self) -> Node {
        Node(Box::new(self.clone()))
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_ref_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A data for synchronization. See [`NodeTrait::sync_native`] for more info.
pub struct SyncContext<'a, 'b> {
    /// A reference to a pool with nodes from a scene graph.
    pub nodes: &'a NodePool,
    /// A mutable reference to 3D physics world.
    pub physics: &'a mut graph::physics::PhysicsWorld,
    /// A mutable reference to 2D physics world.
    pub physics2d: &'a mut dim2::physics::PhysicsWorld,
    /// A mutable reference to sound context.
    pub sound_context: &'a mut SoundContext,
    /// A reference to graph update switches. See [`GraphUpdateSwitches`] for more info.
    pub switches: Option<&'b GraphUpdateSwitches>,
}

/// A data for update tick. See [`NodeTrait::update`] for more info.
pub struct UpdateContext<'a> {
    /// Size of client area of the window.
    pub frame_size: Vector2<f32>,
    /// A time that have passed since last update call.
    pub dt: f32,
    /// A reference to a pool with nodes from a scene graph.
    pub nodes: &'a mut NodePool,
    /// A mutable reference to 3D physics world.
    pub physics: &'a mut graph::physics::PhysicsWorld,
    /// A mutable reference to 2D physics world.
    pub physics2d: &'a mut dim2::physics::PhysicsWorld,
    /// A mutable reference to sound context.
    pub sound_context: &'a mut SoundContext,
}

/// Implements [`NodeTrait::query_component_ref`] and [`NodeTrait::query_component_mut`] in a much
/// shorter way.
#[macro_export]
macro_rules! impl_query_component {
    ($($comp_field:ident: $comp_type:ty),*) => {
        fn query_component_ref(&self, type_id: std::any::TypeId) -> Option<&dyn std::any::Any> {
            if type_id == std::any::TypeId::of::<Self>() {
                return Some(self);
            }

            $(
                if type_id == std::any::TypeId::of::<$comp_type>() {
                    return Some(&self.$comp_field)
                }
            )*

            None
        }

        fn query_component_mut(
            &mut self,
            type_id: std::any::TypeId,
        ) -> Option<&mut dyn std::any::Any> {
            if type_id == std::any::TypeId::of::<Self>() {
                return Some(self);
            }

            $(
                if type_id == std::any::TypeId::of::<$comp_type>() {
                    return Some(&mut self.$comp_field)
                }
            )*

            None
        }
    };
}

/// A main trait for any scene graph node.
pub trait NodeTrait: BaseNodeTrait + Reflect + Visit {
    /// Allows a node to provide access to inner components.
    fn query_component_ref(&self, type_id: TypeId) -> Option<&dyn Any>;

    /// Allows a node to provide access to inner components.
    fn query_component_mut(&mut self, type_id: TypeId) -> Option<&mut dyn Any>;

    /// Returns axis-aligned bounding box in **local space** of the node.
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox;

    /// Returns axis-aligned bounding box in **world space** of the node.
    ///
    /// # Important notes
    ///
    /// World bounding box will become valid **only** after first `update` call of the parent scene.
    /// It is because to calculate world bounding box we must get world transform first, but it
    /// can be calculated with a knowledge of parent world transform, so node on its own cannot know
    /// its world bounding box without additional information.
    fn world_bounding_box(&self) -> AxisAlignedBoundingBox;

    /// Returns actual type id. It will be used for serialization, the type will be saved together
    /// with node's data allowing you to create correct node instance on deserialization.
    fn id(&self) -> Uuid;

    /// Gives an opportunity to perform clean up after the node was extracted from the scene graph
    /// (or deleted).
    fn on_removed_from_graph(&mut self, #[allow(unused_variables)] graph: &mut Graph) {}

    /// Synchronizes internal state of the node with components of scene graph. It has limited usage
    /// and mostly allows you to sync the state of backing entity with the state of the node.
    /// For example the engine use it to sync native rigid body properties after some property was
    /// changed in the [`crate::scene::rigidbody::RigidBody`] node.  
    fn sync_native(
        &self,
        #[allow(unused_variables)] self_handle: Handle<Node>,
        #[allow(unused_variables)] context: &mut SyncContext,
    ) {
    }

    /// Called when node's global transform changes.
    fn sync_transform(
        &self,
        #[allow(unused_variables)] new_global_transform: &Matrix4<f32>,
        _context: &mut SyncContext,
    ) {
    }

    /// The methods is used to manage lifetime of scene nodes, depending on their internal logic.
    fn is_alive(&self) -> bool {
        true
    }

    /// Updates internal state of the node.
    fn update(&mut self, #[allow(unused_variables)] context: &mut UpdateContext) {}

    /// Validates internal state of a scene node. It can check handles validity, if a handle "points"
    /// to a node of particular type, if node's parameters are in range, etc. It's main usage is to
    /// provide centralized diagnostics for scene graph.
    fn validate(&self, #[allow(unused_variables)] scene: &Scene) -> Result<(), String> {
        Ok(())
    }
}

/// A small wrapper over `Handle<Node>`. Its main purpose is to provide a convenient way
/// to handle arrays of handles in the editor.
#[derive(Reflect, Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeHandle(pub Handle<Node>);

impl Deref for NodeHandle {
    type Target = Handle<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Visit for NodeHandle {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

/// Node is the basic building block for 3D scenes. It has multiple variants, but all of them share some
/// common functionality:
///
/// - Local and global [transform](super::transform::Transform)
/// - Info about connections with other nodes in scene
/// - Visibility state - local and global
/// - Name and tags
/// - Level of details
/// - Physics binding mode
///
/// The exact functionality depends on variant of the node, check the respective docs for a variant you
/// interested in.
///
/// # Hierarchy
///
/// Nodes can be connected with other nodes, so a child node will be moved/rotate/scaled together with parent
/// node. This has some analogy in real world - imagine a pen with a cap. The pen will be the parent node in
/// the hierarchy and the cap will be child node. When you moving the pen, the cap moves with it only if it
/// attached to the pen. The same principle works with scene nodes.
///
/// # Transform
///
/// The node has two kinds of transform - local and global. Local transform defines where the node is located
/// (translation) relative to origin, how much it is scaled (in percent) and rotated (around any arbitrary axis).
/// Global transform is almost the same, but it also includes the whole chain of transforms of parent nodes.
/// In the previous example with the pen, the cap has its own local transform which tells how much it should be
/// moved from origin to be exactly on top of the pen. But global transform of the cap includes transform of the
/// pen. So if you move the pen, the local transform of the cap will remain the same, but global transform will
/// include the transform of the pen.
///
/// # Name and tag
///
/// The node can have arbitrary name and tag. Both could be used to search the node in the graph. Unlike the name,
/// tag could be used to store some gameplay information about the node. For example you can place a [`Mesh`] node
/// that represents health pack model and it will have a name "HealthPack", in the tag you could put additional info
/// like "MediumPack", "SmallPack", etc. So 3D model will not have "garbage" in its name, it will be stored inside tag.
///
/// # Visibility
///
/// The now has two kinds of visibility - local and global. As with transform, everything here is pretty similar.
/// Local visibility defines if the node is visible as if it would be rendered alone, global visibility includes
/// the combined visibility of entire chain of parent nodes.
///
/// Please keep in mind that "visibility" here means some sort of a "switch" that tells the renderer whether to draw
/// the node or not. To fetch actual visibility of the node from a camera's perspective, use
/// [visibility cache](super::visibility::VisibilityCache) of the camera.
///
/// # Level of details
///
/// The node could control which children nodes should be drawn based on the distance to a camera, this is so called
/// level of detail functionality. There is a separate article about LODs, it can be found [here](super::base::LevelOfDetail).
#[derive(Debug)]
pub struct Node(Box<dyn NodeTrait>);

impl Deref for Node {
    type Target = dyn NodeTrait;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:ty => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
        /// Returns true if node is instance of given type.
        pub fn $is(&self) -> bool {
            self.cast::<$typ>().is_some()
        }

        /// Tries to cast shared reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_ref(&self) -> &$typ {
            self.cast::<$typ>()
                .unwrap_or_else(|| panic!("Cast to {} failed!", stringify!($kind)))
        }

        /// Tries to cast mutable reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_mut(&mut self) -> &mut $typ {
            self.cast_mut::<$typ>()
                .unwrap_or_else(|| panic!("Cast to {} failed!", stringify!($kind)))
        }
    };
}

impl Node {
    /// Creates a new node instance from any type that implements [`NodeTrait`].
    pub fn new<T: NodeTrait>(node: T) -> Self {
        Self(Box::new(node))
    }

    /// Performs downcasting to a particular type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fyrox::scene::mesh::Mesh;
    /// # use fyrox::scene::node::Node;
    ///
    /// fn node_as_mesh_ref(node: &Node) -> &Mesh {
    ///     node.cast::<Mesh>().expect("Expected to be an instance of Mesh")
    /// }
    /// ```
    pub fn cast<T: NodeTrait>(&self) -> Option<&T> {
        self.0.as_any_ref().downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fyrox::scene::mesh::Mesh;
    /// # use fyrox::scene::node::Node;
    ///
    /// fn node_as_mesh_mut(node: &mut Node) -> &mut Mesh {
    ///     node.cast_mut::<Mesh>().expect("Expected to be an instance of Mesh")
    /// }
    /// ```
    pub fn cast_mut<T: NodeTrait>(&mut self) -> Option<&mut T> {
        self.0.as_any_ref_mut().downcast_mut::<T>()
    }

    /// Allows a node to provide access to a component of specified type.
    ///
    /// # Example
    ///
    /// A good example is a light source node, it gives access to internal `BaseLight`:
    ///
    /// ```rust
    /// # use fyrox::scene::light::BaseLight;
    /// # use fyrox::scene::light::directional::DirectionalLight;
    /// # use fyrox::scene::node::{Node};
    ///  
    /// fn base_light_ref(directional_light: &Node) -> &BaseLight {
    ///     directional_light.query_component_ref::<BaseLight>().expect("Must have base light")
    /// }
    ///
    /// ```
    ///
    /// Some nodes could also provide access to inner components, check documentation of a node.
    pub fn query_component_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .query_component_ref(TypeId::of::<T>())
            .and_then(|c| c.downcast_ref::<T>())
    }

    /// Allows a node to provide access to a component of specified type.
    ///
    /// # Example
    ///
    /// A good example is a light source node, it gives access to internal `BaseLight`:
    ///
    /// ```rust
    /// # use fyrox::scene::light::BaseLight;
    /// # use fyrox::scene::light::directional::DirectionalLight;
    /// # use fyrox::scene::node::{Node};
    ///  
    /// fn base_light_mut(directional_light: &mut Node) -> &mut BaseLight {
    ///     directional_light.query_component_mut::<BaseLight>().expect("Must have base light")
    /// }
    ///
    /// ```
    ///
    /// Some nodes could also provide access to inner components, check documentation of a node.
    pub fn query_component_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0
            .query_component_mut(TypeId::of::<T>())
            .and_then(|c| c.downcast_mut::<T>())
    }

    define_is_as!(Mesh => fn is_mesh, fn as_mesh, fn as_mesh_mut);
    define_is_as!(Camera  => fn is_camera, fn as_camera, fn as_camera_mut);
    define_is_as!(SpotLight  => fn is_spot_light, fn as_spot_light, fn as_spot_light_mut);
    define_is_as!(PointLight  => fn is_point_light, fn as_point_light, fn as_point_light_mut);
    define_is_as!(PointLight  => fn is_directional_light, fn as_directional_light, fn as_directional_light_mut);
    define_is_as!(ParticleSystem => fn is_particle_system, fn as_particle_system, fn as_particle_system_mut);
    define_is_as!(Sprite  => fn is_sprite, fn as_sprite, fn as_sprite_mut);
    define_is_as!(Terrain  => fn is_terrain, fn as_terrain, fn as_terrain_mut);
    define_is_as!(Decal => fn is_decal, fn as_decal, fn as_decal_mut);
    define_is_as!(Rectangle => fn is_rectangle, fn as_rectangle, fn as_rectangle_mut);
    define_is_as!(scene::rigidbody::RigidBody  => fn is_rigid_body, fn as_rigid_body, fn as_rigid_body_mut);
    define_is_as!(scene::collider::Collider => fn is_collider, fn as_collider, fn as_collider_mut);
    define_is_as!(scene::joint::Joint  => fn is_joint, fn as_joint, fn as_joint_mut);
    define_is_as!(dim2::rigidbody::RigidBody => fn is_rigid_body2d, fn as_rigid_body2d, fn as_rigid_body2d_mut);
    define_is_as!(dim2::collider::Collider => fn is_collider2d, fn as_collider2d, fn as_collider2d_mut);
    define_is_as!(dim2::joint::Joint => fn is_joint2d, fn as_joint2d, fn as_joint2d_mut);
    define_is_as!(Sound => fn is_sound, fn as_sound, fn as_sound_mut);
    define_is_as!(Listener => fn is_listener, fn as_listener, fn as_listener_mut);
    define_is_as!(NavigationalMesh => fn is_navigational_mesh, fn as_navigational_mesh, fn as_navigational_mesh_mut);
}

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Reflect for Node {
    fn type_name(&self) -> &'static str {
        self.0.deref().type_name()
    }

    fn fields_info(&self, func: &mut dyn FnMut(Vec<FieldInfo>)) {
        self.0.deref().fields_info(func)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self.0.into_any()
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        self.0.deref().as_any(func)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        self.0.deref_mut().as_any_mut(func)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        self.0.deref().as_reflect(func)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        self.0.deref_mut().as_reflect_mut(func)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.0.deref_mut().set(value)
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
    ) {
        self.0.deref_mut().set_field(field, value, func)
    }

    fn fields(&self, func: &mut dyn FnMut(Vec<&dyn Reflect>)) {
        self.0.deref().fields(func)
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(Vec<&mut dyn Reflect>)) {
        self.0.deref_mut().fields_mut(func)
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.0.deref().field(name, func)
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.0.deref_mut().field_mut(name, func)
    }
}
