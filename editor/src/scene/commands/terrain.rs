use crate::{command::Command, create_terrain_layer_material, scene::commands::SceneContext};
use fyrox::{
    core::pool::Handle,
    resource::texture::Texture,
    scene::{node::Node, terrain::Layer},
};

#[derive(Debug)]
pub struct AddTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
    masks: Vec<Texture>,
}

impl AddTerrainLayerCommand {
    pub fn new(terrain_handle: Handle<Node>) -> Self {
        Self {
            terrain: terrain_handle,
            layer: Some(Layer {
                material: create_terrain_layer_material(),
                mask_property_name: "maskTexture".to_owned(),
            }),
            masks: Default::default(),
        }
    }
}

impl Command for AddTerrainLayerCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        terrain.add_layer(self.layer.take().unwrap(), std::mem::take(&mut self.masks));
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        let (layer, masks) = terrain.pop_layer().unwrap();
        self.layer = Some(layer);
        self.masks = masks;
    }
}

#[derive(Debug)]
pub struct DeleteTerrainLayerCommand {
    terrain: Handle<Node>,
    layer: Option<Layer>,
    index: usize,
    masks: Vec<Texture>,
}

impl DeleteTerrainLayerCommand {
    pub fn new(terrain: Handle<Node>, index: usize) -> Self {
        Self {
            terrain,
            layer: Default::default(),
            index,
            masks: Default::default(),
        }
    }
}

impl Command for DeleteTerrainLayerCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Terrain Layer".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let (layer, masks) = context.scene.graph[self.terrain]
            .as_terrain_mut()
            .remove_layer(self.index);

        self.layer = Some(layer);
        self.masks = masks;
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        terrain.insert_layer(
            self.layer.take().unwrap(),
            std::mem::take(&mut self.masks),
            self.index,
        );
    }
}

#[derive(Debug)]
pub struct ModifyTerrainHeightCommand {
    terrain: Handle<Node>,
    // TODO: This is very memory-inefficient solution, it could be done
    //  better by either pack/unpack data on the fly, or by saving changes
    //  for sub-chunks.
    old_heightmaps: Vec<Vec<f32>>,
    new_heightmaps: Vec<Vec<f32>>,
}

impl ModifyTerrainHeightCommand {
    pub fn new(
        terrain: Handle<Node>,
        old_heightmaps: Vec<Vec<f32>>,
        new_heightmaps: Vec<Vec<f32>>,
    ) -> Self {
        Self {
            terrain,
            old_heightmaps,
            new_heightmaps,
        }
    }

    pub fn swap(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();
        for (chunk, (old, new)) in terrain.chunks_mut().iter_mut().zip(
            self.old_heightmaps
                .iter_mut()
                .zip(self.new_heightmaps.iter_mut()),
        ) {
            chunk.set_heightmap(new.clone());
            std::mem::swap(old, new);
        }
    }
}

impl Command for ModifyTerrainHeightCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Modify Terrain Height".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct ModifyTerrainLayerMaskCommand {
    terrain: Handle<Node>,
    // TODO: This is very memory-inefficient solution, it could be done
    //  better by either pack/unpack data on the fly, or by saving changes
    //  for sub-chunks.
    old_masks: Vec<Vec<u8>>,
    new_masks: Vec<Vec<u8>>,
    layer: usize,
}

impl ModifyTerrainLayerMaskCommand {
    pub fn new(
        terrain: Handle<Node>,
        old_masks: Vec<Vec<u8>>,
        new_masks: Vec<Vec<u8>>,
        layer: usize,
    ) -> Self {
        Self {
            terrain,
            old_masks,
            new_masks,
            layer,
        }
    }

    pub fn swap(&mut self, context: &mut SceneContext) {
        let terrain = context.scene.graph[self.terrain].as_terrain_mut();

        for (i, chunk) in terrain.chunks_mut().iter_mut().enumerate() {
            let old = &mut self.old_masks[i];
            let new = &mut self.new_masks[i];
            let chunk_mask = &mut chunk.layer_masks[self.layer];

            let mut texture_data = chunk_mask.data_ref();

            for (mask_pixel, new_pixel) in
                texture_data.modify().data_mut().iter_mut().zip(new.iter())
            {
                *mask_pixel = *new_pixel;
            }

            std::mem::swap(old, new);
        }
    }
}

impl Command for ModifyTerrainLayerMaskCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Modify Terrain Layer Mask".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }
}
