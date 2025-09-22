use crate::{
    tile_component::{BaseTerrain, Feature, Resource, TerrainType},
    tile_map::TileMap,
};

impl TileMap {
    /// Fix Sugar graphics. That because in origin CIV5, `Sugar` could not be made visible enough in jungle, so turn any sugar jungle to marsh.
    ///
    /// Change all the terrains which both have [`Feature::Jungle`] and resource `Sugar` to a [`TerrainType::Flatland`]
    /// with [`BaseTerrain::Grassland`] and [`Feature::Marsh`].
    pub fn fix_sugar_jungles(&mut self) {
        self.all_tiles().for_each(|tile| {
            if tile
                .resource(self)
                .is_some_and(|(resource, _)| resource == Resource::Sugar)
                && tile.feature(self) == Some(Feature::Jungle)
            {
                tile.set_terrain_type(self, TerrainType::Flatland);
                tile.set_base_terrain(self, BaseTerrain::Grassland);
                tile.set_feature(self, Feature::Marsh);
            }
        })
    }
}
