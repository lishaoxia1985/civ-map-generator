use crate::{
    grid::{Grid, offset_coordinate::OffsetCoordinate},
    tile::Tile,
    tile_component::TerrainType,
    tile_map::TileMap,
};

impl TileMap {
    /// Shift terrain types to align the most water-heavy portions of the map with the edges.
    ///
    /// This is only done if the map wraps around the respective axis.
    pub fn shift_terrain_types(&mut self) {
        let grid = self.world_grid.grid;

        // No need to shift if the map doesn't wrap.
        if !grid.wrap_x() && !grid.wrap_y() {
            return;
        }

        let x_shift = if grid.wrap_x() {
            self.determine_x_shift()
        } else {
            0
        };
        let y_shift = if grid.wrap_y() {
            self.determine_y_shift()
        } else {
            0
        };

        if x_shift == 0 && y_shift == 0 {
            return;
        }

        let mut terrain_type_list = Vec::with_capacity((grid.width() * grid.height()) as usize);
        // Populate new terrain type list with shifted values
        for y in 0..grid.height() as i32 {
            for x in 0..grid.width() as i32 {
                let source_x = x + x_shift;
                let source_y = y + y_shift;
                let source_tile =
                    Tile::from_offset(OffsetCoordinate::new(source_x, source_y), grid);
                terrain_type_list.push(source_tile.terrain_type(self));
            }
        }

        self.terrain_type_list = terrain_type_list;
    }

    fn determine_x_shift(&mut self) -> i32 {
        // This function aligns the most water-heavy vertical portion of the map with the vertical map edge.
        // It looks at groups of columns and picks the center of the most water-heavy group as the new edge.

        // First calculate land totals for each column
        let grid = self.world_grid.grid;
        let mut land_totals = vec![0; grid.width() as usize];

        self.all_tiles().for_each(|tile| {
            let [x, _] = tile.to_offset(grid).to_array();
            if tile.terrain_type(self) != TerrainType::Water {
                land_totals[x as usize] += 1;
            }
        });

        // Evaluate column groups
        let group_radius = (grid.width() / 10).max(1) as i32; // Ensure at least 1
        let mut column_groups = vec![0; grid.width() as usize];

        for column_index in 0..grid.width() as i32 {
            let mut current_group_total = 0;

            for offset in -group_radius..=group_radius {
                let mut current_column = column_index + offset;

                // Handle wrap-around for circular map
                if current_column < 0 {
                    current_column += grid.width() as i32;
                } else if current_column >= grid.width() as i32 {
                    current_column -= grid.width() as i32;
                }

                current_group_total += land_totals[current_column as usize];
            }

            column_groups[column_index as usize] = current_group_total;
        }

        // Find group with least land (most water)
        let (best_group, _) = column_groups
            .iter()
            .enumerate()
            .min_by_key(|&(_, &group_land_tiles)| group_land_tiles)
            .expect("The map is empty. This should never happen.");

        // Return x shift (converting from usize to i32)
        best_group as i32
    }

    fn determine_y_shift(&mut self) -> i32 {
        // This function aligns the most water-heavy horizontal portion of the map with the horizontal map edge.
        // It looks at groups of rows and picks the center of the most water-heavy group as the new edge.

        // First calculate land totals for each row
        let grid = self.world_grid.grid;
        let mut land_totals = vec![0; grid.height() as usize];

        self.all_tiles().for_each(|tile| {
            let [_, y] = tile.to_offset(grid).to_array();
            if tile.terrain_type(self) != TerrainType::Water {
                land_totals[y as usize] += 1;
            }
        });

        // Evaluate row groups
        let group_radius = (grid.height() / 15).max(1) as i32; // Ensure at least 1
        let mut row_groups = vec![0; grid.height() as usize];

        for row_index in 0..grid.height() as i32 {
            let mut current_group_total = 0;

            for offset in -group_radius..=group_radius {
                let mut current_row = row_index + offset;

                // Handle wrap-around for circular map
                if current_row < 0 {
                    current_row += grid.height() as i32;
                } else if current_row >= grid.height() as i32 {
                    current_row -= grid.height() as i32;
                }

                current_group_total += land_totals[current_row as usize];
            }

            row_groups[row_index as usize] = current_group_total;
        }

        // Find group with least land (most water)
        let best_group = row_groups
            .iter()
            .enumerate()
            .min_by_key(|&(_, &group_land_tiles)| group_land_tiles)
            .expect("The map is empty. This should never happen.")
            .0;

        // Return y shift (converting from usize to i32)
        best_group as i32
    }
}
