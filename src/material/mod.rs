//! Material is a set of parameters for a shader. This module contains everything related to materials.
//!
//! See [Material struct docs](self::Material) for more info.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        parking_lot::{Mutex, MutexGuard},
        reflect::prelude::*,
        sstorage::ImmutableString,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    material::shader::{PropertyKind, SamplerFallback, Shader},
    resource::texture::Texture,
};
use fxhash::FxHashMap;
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
    sync::Arc,
};

pub mod shader;

/// A value of a property that will be used for rendering with a shader.
///
/// # Limitations
///
/// There is a limited set of possible types that can be passed to a shader, most of them are
/// just simple data types.
#[derive(Debug, Visit, Clone, Reflect)]
pub enum PropertyValue {
    /// Real number.
    Float(f32),

    /// Real number array.
    FloatArray(Vec<f32>),

    /// Integer number.
    Int(i32),

    /// Integer number array.
    IntArray(Vec<i32>),

    /// Natural number.
    UInt(u32),

    /// Natural number array.
    UIntArray(Vec<u32>),

    /// Two-dimensional vector.
    Vector2(Vector2<f32>),

    /// Two-dimensional vector array.
    Vector2Array(Vec<Vector2<f32>>),

    /// Three-dimensional vector.
    Vector3(Vector3<f32>),

    /// Three-dimensional vector array.
    Vector3Array(Vec<Vector3<f32>>),

    /// Four-dimensional vector.
    Vector4(Vector4<f32>),

    /// Four-dimensional vector array.
    Vector4Array(Vec<Vector4<f32>>),

    /// 2x2 Matrix.
    Matrix2(Matrix2<f32>),

    /// 2x2 Matrix array.
    Matrix2Array(Vec<Matrix2<f32>>),

    /// 3x3 Matrix.
    Matrix3(Matrix3<f32>),

    /// 3x3 Matrix array.
    Matrix3Array(Vec<Matrix3<f32>>),

    /// 4x4 Matrix.
    Matrix4(Matrix4<f32>),

    /// 4x4 Matrix array.
    Matrix4Array(Vec<Matrix4<f32>>),

    /// Boolean value.
    Bool(bool),

    /// An sRGB color.
    ///
    /// # Conversion
    ///
    /// The colors you see on your monitor are in sRGB color space, this is fine for simple cases
    /// of rendering, but not for complex things like lighting. Such things require color to be
    /// linear. Value of this variant will be automatically **converted to linear color space**
    /// before it passed to shader.
    Color(Color),

    /// A texture with fallback option.
    ///
    /// # Fallback
    ///
    /// Sometimes you don't want to set a value to a sampler, or you even don't have the appropriate
    /// one. There is fallback value that helps you with such situations, it defines a values that
    /// will be fetched from a sampler when there is no texture.
    ///
    /// For example, standard shader has a lot of samplers defined: diffuse, normal, height, emission,
    /// mask, metallic, roughness, etc. In some situations you may not have all the textures, you have
    /// only diffuse texture, to keep rendering correct, each other property has appropriate fallback
    /// value. Normal sampler - a normal vector pointing up (+Y), height - zero, emission - zero, etc.
    ///
    /// Fallback value is also helpful to catch missing textures, you'll definitely know the texture is
    /// missing by very specific value in the fallback texture.
    Sampler {
        /// Actual value of the sampler. Could be [`None`], in this case `fallback` will be used.
        value: Option<Texture>,

        /// Sampler fallback value.
        fallback: SamplerFallback,
    },
}

macro_rules! define_as {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<$ty> {
            if let PropertyValue::$variant(v) = self {
                Some(*v)
            } else {
                None
            }
        }
    };
}

macro_rules! define_as_ref {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<&$ty> {
            if let PropertyValue::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
}

impl PropertyValue {
    define_as!(
        /// Tries to unwrap property value as float.
        as_float = Float -> f32
    );
    define_as_ref!(
        /// Tries to unwrap property value as float array.
        as_float_array = FloatArray -> [f32]
    );
    define_as!(
        /// Tries to unwrap property value as integer.
        as_int = Int -> i32
    );
    define_as_ref!(
        /// Tries to unwrap property value as integer array.
        as_int_array = IntArray -> [i32]
    );
    define_as!(
        /// Tries to unwrap property value as unsigned integer.
        as_uint = UInt -> u32
    );
    define_as_ref!(
        /// Tries to unwrap property value as unsigned integer array.
        as_uint_array = UIntArray -> [u32]
    );
    define_as!(
        /// Tries to unwrap property value as boolean.
        as_bool = Bool -> bool
    );
    define_as!(
        /// Tries to unwrap property value as color.
        as_color = Color -> Color
    );
    define_as!(
        /// Tries to unwrap property value as two-dimensional vector.
        as_vector2 = Vector2 -> Vector2<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as two-dimensional vector array.
        as_vector2_array = Vector2Array -> [Vector2<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as three-dimensional vector.
        as_vector3 = Vector3 -> Vector3<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as three-dimensional vector array.
        as_vector3_array = Vector3Array -> [Vector3<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as four-dimensional vector.
        as_vector4 = Vector4 -> Vector4<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as four-dimensional vector array.
        as_vector4_array = Vector4Array -> [Vector4<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 2x2 matrix.
        as_matrix2 = Matrix2 -> Matrix2<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 2x2 matrix array.
        as_matrix2_array = Matrix2Array -> [Matrix2<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 3x3 matrix.
        as_matrix3 = Matrix3 -> Matrix3<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 3x3 matrix array.
        as_matrix3_array = Matrix3Array -> [Matrix3<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 4x4 matrix.
        as_matrix4 = Matrix4 -> Matrix4<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 4x4 matrix array.
        as_matrix4_array = Matrix4Array -> [Matrix4<f32>]
    );

    /// Tries to unwrap property value as texture.
    pub fn as_sampler(&self) -> Option<Texture> {
        if let PropertyValue::Sampler { value, .. } = self {
            value.clone()
        } else {
            None
        }
    }
}

impl Default for PropertyValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Material defines a set of values for a shader. Materials usually contains textures (diffuse,
/// normal, height, emission, etc. maps), numerical values (floats, integers), vectors, booleans,
/// matrices and arrays of each type, except textures. Each parameter can be changed in runtime
/// giving you the ability to create animated materials. However in practice, most materials are
/// static, this means that once it created, it won't be changed anymore.
///
/// Please keep in mind that the actual "rules" of drawing an entity are stored in the shader,
/// **material is only a storage** for specific uses of the shader.
///
/// Multiple materials can share the same shader, for example standard shader covers 95% of most
/// common use cases and it is shared across multiple materials. The only difference are property
/// values, for example you can draw multiple cubes using the same shader, but with different
/// textures.
///
/// Material itself can be shared across multiple places as well as the shader. This gives you the
/// ability to render multiple objects with the same material efficiently.
///
/// # Performance
///
/// It is very important re-use materials as much as possible, because the amount of materials used
/// per frame significantly correlates with performance. The more unique materials you have per frame,
/// the more work has to be done by the renderer and video driver to render a frame and the more time
/// the frame will require for rendering, thus lowering your FPS.
///
/// # Examples
///
/// A material can only be created using a shader instance, every material must have a shader. The
/// shader provides information about its properties, and this information is used to populate a set
/// of properties with default values. Default values of each property defined in the shader.
///
/// ## Standard material
///
/// Usually standard shader is enough for most cases, [`Material`] even has a [`Material::standard()`]
/// method to create a material with standard shader:
///
/// ```no_run
/// use fyrox::{
///     material::shader::{Shader, SamplerFallback},
///     engine::resource_manager::ResourceManager,
///     material::{Material, PropertyValue},
///     core::sstorage::ImmutableString,
/// };
///
/// fn create_brick_material(resource_manager: ResourceManager) -> Material {
///     let mut material = Material::standard();
///
///     material.set_property(
///         &ImmutableString::new("diffuseTexture"),
///         PropertyValue::Sampler {
///             value: Some(resource_manager.request_texture("Brick_DiffuseTexture.jpg")),
///             fallback: SamplerFallback::White
///         })
///         .unwrap();
///
///     material
/// }
/// ```
///
/// As you can see it is pretty simple with standard material, all you need is to set values to desired
/// properties and you good to go. All you need to do is to apply the material, for example it could be
/// mesh surface or some other place that supports materials. For the full list of properties of the
/// standard shader see [shader module docs](self::shader).
///
/// ## Custom material
///
/// Custom materials is a bit more complex, you need to get a shader instance using the resource manager
/// and then create the material and populate it with a set of property values.
///
/// ```no_run
/// use fyrox::{
///     engine::resource_manager::ResourceManager,
///     material::{Material, PropertyValue},
///     core::{sstorage::ImmutableString, algebra::Vector3}
/// };
///
/// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
///     let shader = resource_manager.request_shader("my_grass_shader.ron").await.unwrap();
///
///     // Here we assume that the material really has the properties defined below.
///     let mut material = Material::from_shader(shader, Some(resource_manager));
///
///     material.set_property(
///         &ImmutableString::new("windDirection"),
///         PropertyValue::Vector3(Vector3::new(1.0, 0.0, 0.5))
///         )
///         .unwrap();
///
///     material
/// }
/// ```
///
/// As you can see it is only a bit more hard that with the standard shader. The main difference here is
/// that we using resource manager to get shader instance and the we just use the instance to create
/// material instance. Then we populate properties as usual.
#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct Material {
    shader: Shader,
    properties: FxHashMap<ImmutableString, PropertyValue>,
}

/// A set of possible errors that can occur when working with materials.
#[derive(Clone, Debug)]
pub enum MaterialError {
    /// A property is missing.
    NoSuchProperty {
        /// Name of the property.
        property_name: String,
    },

    /// Attempt to set a value of wrong type to a property.
    TypeMismatch {
        /// Name of the property.
        property_name: String,
        /// Expected property value.
        expected: PropertyValue,
        /// Given property value.
        given: PropertyValue,
    },
}

impl Display for MaterialError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialError::NoSuchProperty { property_name } => {
                write!(f, "Unable to find material property {property_name}")
            }
            MaterialError::TypeMismatch {
                property_name,
                expected,
                given,
            } => {
                write!(
                    f,
                    "Attempt to set a value of wrong type \
                to {property_name} property. Expected: {expected:?}, given {given:?}"
                )
            }
        }
    }
}

impl Material {
    /// Creates a new instance of material with the standard shader. For the full list
    /// of properties of the standard material see [shader module docs](self::shader).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fyrox::{
    ///     material::shader::{Shader, SamplerFallback},
    ///     engine::resource_manager::ResourceManager,
    ///     material::{Material, PropertyValue},
    ///     core::sstorage::ImmutableString
    /// };
    ///
    /// fn create_brick_material(resource_manager: ResourceManager) -> Material {
    ///     let mut material = Material::standard();
    ///
    ///     material.set_property(
    ///         &ImmutableString::new("diffuseTexture"),
    ///         PropertyValue::Sampler {
    ///             value: Some(resource_manager.request_texture("Brick_DiffuseTexture.jpg")),
    ///             fallback: SamplerFallback::White
    ///         })
    ///         .unwrap();
    ///
    ///     material
    /// }
    /// ```
    pub fn standard() -> Self {
        Self::from_shader(Shader::standard(), None)
    }

    /// Creates new instance of standard terrain material.
    pub fn standard_terrain() -> Self {
        Self::from_shader(Shader::standard_terrain(), None)
    }

    /// Creates a new material instance with given shader. Each property will have default values
    /// defined in the shader.
    ///
    /// It is possible to pass resource manager as a second argument, it is needed to correctly resolve
    /// default values of samplers in case if they are bound to some resources - shader's definition stores
    /// only paths to textures. If you pass [`None`], no resolving will be done and every sampler will
    /// have [`None`] as default value, which in its turn will force engine to use fallback sampler value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fyrox::{
    ///     engine::resource_manager::ResourceManager,
    ///     material::{Material, PropertyValue},
    ///     core::{sstorage::ImmutableString, algebra::Vector3}
    /// };
    ///
    /// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
    ///     let shader = resource_manager.request_shader("my_grass_shader.ron").await.unwrap();
    ///
    ///     // Here we assume that the material really has the properties defined below.
    ///     let mut material = Material::from_shader(shader, Some(resource_manager));
    ///
    ///     material.set_property(
    ///         &ImmutableString::new("windDirection"),
    ///         PropertyValue::Vector3(Vector3::new(1.0, 0.0, 0.5))
    ///         )
    ///         .unwrap();
    ///
    ///     material
    /// }
    /// ```
    pub fn from_shader(shader: Shader, resource_manager: Option<ResourceManager>) -> Self {
        let data = shader.data_ref();

        let mut property_values = FxHashMap::default();
        for property_definition in data.definition.properties.iter() {
            let value = match &property_definition.kind {
                PropertyKind::Float(value) => PropertyValue::Float(*value),
                PropertyKind::Int(value) => PropertyValue::Int(*value),
                PropertyKind::UInt(value) => PropertyValue::UInt(*value),
                PropertyKind::Vector2(value) => PropertyValue::Vector2(*value),
                PropertyKind::Vector3(value) => PropertyValue::Vector3(*value),
                PropertyKind::Vector4(value) => PropertyValue::Vector4(*value),
                PropertyKind::Color { r, g, b, a } => {
                    PropertyValue::Color(Color::from_rgba(*r, *g, *b, *a))
                }
                PropertyKind::Matrix2(value) => PropertyValue::Matrix2(*value),
                PropertyKind::Matrix3(value) => PropertyValue::Matrix3(*value),
                PropertyKind::Matrix4(value) => PropertyValue::Matrix4(*value),
                PropertyKind::Bool(value) => PropertyValue::Bool(*value),
                PropertyKind::Sampler {
                    default,
                    fallback: usage,
                } => PropertyValue::Sampler {
                    value: default.as_ref().and_then(|path| {
                        resource_manager.clone().map(|rm| rm.request_texture(path))
                    }),
                    fallback: *usage,
                },
                PropertyKind::FloatArray(value) => PropertyValue::FloatArray(value.clone()),
                PropertyKind::IntArray(value) => PropertyValue::IntArray(value.clone()),
                PropertyKind::UIntArray(value) => PropertyValue::UIntArray(value.clone()),
                PropertyKind::Vector2Array(value) => PropertyValue::Vector2Array(value.clone()),
                PropertyKind::Vector3Array(value) => PropertyValue::Vector3Array(value.clone()),
                PropertyKind::Vector4Array(value) => PropertyValue::Vector4Array(value.clone()),
                PropertyKind::Matrix2Array(value) => PropertyValue::Matrix2Array(value.clone()),
                PropertyKind::Matrix3Array(value) => PropertyValue::Matrix3Array(value.clone()),
                PropertyKind::Matrix4Array(value) => PropertyValue::Matrix4Array(value.clone()),
            };

            property_values.insert(ImmutableString::new(&property_definition.name), value);
        }

        drop(data);

        Self {
            shader,
            properties: property_values,
        }
    }

    /// Searches for a property with given name.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use fyrox::core::sstorage::ImmutableString;
    /// use fyrox::material::Material;
    ///
    /// let mut material = Material::standard();
    ///
    /// let color = material.property_ref(&ImmutableString::new("diffuseColor")).unwrap().as_color();
    /// ```
    pub fn property_ref(&self, name: &ImmutableString) -> Option<&PropertyValue> {
        self.properties.get(name)
    }

    /// Sets new value of the property with given name.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated.
    /// This helps to catch subtle bugs when you passing "almost" identical values to shader, like
    /// signed and unsigned integers - both have positive values, but GPU is very strict of what
    /// it expects as input value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox::material::{Material, PropertyValue};
    /// # use fyrox::core::color::Color;
    /// # use fyrox::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// assert!(material.set_property(&ImmutableString::new("diffuseColor"), PropertyValue::Color(Color::WHITE)).is_ok());
    /// ```
    pub fn set_property(
        &mut self,
        name: &ImmutableString,
        new_value: PropertyValue,
    ) -> Result<(), MaterialError> {
        if let Some(value) = self.properties.get_mut(name) {
            match (value, new_value) {
                (
                    PropertyValue::Sampler {
                        value: old_value,
                        fallback: old_fallback,
                    },
                    PropertyValue::Sampler { value, fallback },
                ) => {
                    *old_value = value;
                    *old_fallback = fallback;
                }
                (PropertyValue::Float(old_value), PropertyValue::Float(value)) => {
                    *old_value = value;
                }
                (PropertyValue::FloatArray(old_value), PropertyValue::FloatArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Int(old_value), PropertyValue::Int(value)) => {
                    *old_value = value;
                }
                (PropertyValue::IntArray(old_value), PropertyValue::IntArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Bool(old_value), PropertyValue::Bool(value)) => {
                    *old_value = value;
                }
                (PropertyValue::UInt(old_value), PropertyValue::UInt(value)) => {
                    *old_value = value;
                }
                (PropertyValue::UIntArray(old_value), PropertyValue::UIntArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector2(old_value), PropertyValue::Vector2(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector2Array(old_value), PropertyValue::Vector2Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector3(old_value), PropertyValue::Vector3(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector3Array(old_value), PropertyValue::Vector3Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector4(old_value), PropertyValue::Vector4(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector4Array(old_value), PropertyValue::Vector4Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix2(old_value), PropertyValue::Matrix2(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix2Array(old_value), PropertyValue::Matrix2Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix3(old_value), PropertyValue::Matrix3(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix3Array(old_value), PropertyValue::Matrix3Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix4(old_value), PropertyValue::Matrix4(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix4Array(old_value), PropertyValue::Matrix4Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Color(old_value), PropertyValue::Color(value)) => {
                    *old_value = value;
                }
                (value, new_value) => {
                    return Err(MaterialError::TypeMismatch {
                        property_name: name.deref().to_owned(),
                        expected: value.clone(),
                        given: new_value,
                    })
                }
            }

            Ok(())
        } else {
            Err(MaterialError::NoSuchProperty {
                property_name: name.deref().to_owned(),
            })
        }
    }

    /// Returns a reference to current shader.
    pub fn shader(&self) -> &Shader {
        &self.shader
    }

    /// Returns immutable reference to internal property storage.
    pub fn properties(&self) -> &FxHashMap<ImmutableString, PropertyValue> {
        &self.properties
    }
}

/// Shared material is a material instance that can be used across multiple objects. It is useful
/// when you need to have multiple objects that have the same material.
///
/// Shared material is also tells a renderer that this material can be used for efficient rendering -
/// the renderer will be able to optimize rendering when it knows that multiple objects share the
/// same material.
#[derive(Reflect, Clone, Debug)]
pub struct SharedMaterial(Arc<Mutex<Material>>);

impl Default for SharedMaterial {
    fn default() -> Self {
        Self::new(Material::standard())
    }
}

impl PartialEq for SharedMaterial {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Visit for SharedMaterial {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl SharedMaterial {
    /// Creates new shared material from a material instance.
    pub fn new(material: Material) -> Self {
        Self(Arc::new(Mutex::new(material)))
    }

    /// Provides access to inner material.
    pub fn lock(&self) -> MutexGuard<'_, Material> {
        self.0.lock()
    }

    /// Returns unique id of the material. The id is not stable across multiple runs of an application!
    pub fn key(&self) -> u64 {
        &*self.0 as *const _ as u64
    }

    /// Returns total use count of the material.
    pub fn use_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    /// Creates a deep copy of shared material, making "unique" clone of the underlying material.
    /// It is useful when you need to create unique version of a material and set its properties
    /// to some specific values and assign it to an object.
    pub fn deep_copy(&self) -> Self {
        Self::new(self.0.lock().clone())
    }
}
