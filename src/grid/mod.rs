//! This module provides various grid implementations and utilities.
//! It includes support for different grid types such as hexagonal and square grids,
//! calculating distances, neighbors, and converting between grid coordinates and offset coordinates.

use direction::Direction;
use offset_coordinate::OffsetCoordinate;

use bitflags::bitflags;

pub mod direction;
pub mod hex_grid;
pub mod offset_coordinate;
pub mod square_grid;

/// Grid trait defines the interface for a grid structure.
///
/// Grid uses [`Cell`] representing each unique position in the grid.
/// Grids implement this trait with their specific coordinate types,
/// such as [`Hex`](hex_grid::hex::Hex) for hexagonal grids or [`Square`](square_grid::square::Square) for square grids.
/// their specific coordinate types is used to calculate the neighbors of a given coordinate,
/// distance between coordinates, and other grid-related operations.
pub trait Grid {
    /// The type of coordinate used in the grid.
    /// This type is used to calculate neighbors, distances, and other grid-related operations.
    /// For a hex grid, this would be [`Hex`](hex_grid::hex::Hex).
    /// For a square grid, this would be [`Square`](square_grid::square::Square).
    type GridCoordinateType;

    /// The type of the edge direction and corner array.
    /// It should be `[Direction; N]` where N is the number of edges or corners.
    /// For a hex grid, this would be `[Direction; 6]`.
    /// For a square grid, this would be `[Direction; 4]`.
    type DirectionArrayType;

    /// Returns the array of directions for the edges of the grid.
    fn edge_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the array of directions for the corners of the grid.
    fn corner_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the size of the grid as a [`Size`] struct, which contains the width and height of the grid.
    fn size(&self) -> Size;

    /// Returns the width of the grid.
    fn width(&self) -> u32 {
        self.size().width
    }

    /// Returns the height of the grid.
    fn height(&self) -> u32 {
        self.size().height
    }

    /// Returns the flags that indicate how a grid/map wraps at its borders.
    fn wrap_flags(&self) -> WrapFlags;

    /// Returns if the grid is wrapped in the X direction.
    fn wrap_x(&self) -> bool {
        self.wrap_flags().contains(WrapFlags::WrapX)
    }

    /// Returns if the grid is wrapped in the Y direction.
    fn wrap_y(&self) -> bool {
        self.wrap_flags().contains(WrapFlags::WrapY)
    }

    /// Get the center of the grid in pixel coordinates.
    ///
    /// That is often used to center the camera on the grid.
    /// But in the game, the camera is not exactly at the center of the map.
    /// So you might want to adjust the camera position slightly.
    ///
    /// # Notice
    ///
    /// For wrapped grids, returns the hypothetical unwrapped center position
    /// (since wrapped grids have no true center).
    fn center(&self) -> [f32; 2];

    /// Get the left-bottom position of the grid in pixel coordinates.
    ///
    /// That is often used to limit the camera position.
    fn left_bottom(&self) -> [f32; 2];

    /// Get the right-top position of the grid in pixel coordinates.
    ///
    /// That is often used to limit the camera position.
    fn right_top(&self) -> [f32; 2];

    /// Returns the pixel coordinates of the center of the given `OffsetCoordinate`.
    ///
    /// # Arguments
    ///
    /// - `offset`: The offset coordinate to convert. It can be any offset coordinate for wrapped grids,
    ///   but for non-wrapped grids, it must be within the grid bounds.
    ///
    fn offset_to_pixel(&self, offset: OffsetCoordinate) -> [f32; 2];

    /// Returns the `OffsetCoordinate` which contains the given pixel coordinates.
    ///
    /// # Arguments
    ///
    /// - `pixel`: The pixel coordinate to convert. It can be any pixel coordinate for wrapped grids,
    ///   but for non-wrapped grids, it must be within the grid bounds.
    ///
    fn pixel_to_offset(&self, pixel: [f32; 2]) -> OffsetCoordinate;

    /// Converts a `Cell` to an `OffsetCoordinate`.
    ///
    /// `OffsetCoordinate` is a normalized coordinate that fits within the grid's bounds.
    ///
    /// # Panics
    ///
    /// If the cell is out of bounds, it will panic in debug mode.
    fn cell_to_offset(&self, cell: Cell) -> OffsetCoordinate {
        let width = self.size().width;
        let height = self.size().height;

        debug_assert!(
            cell.0 < (width * height) as usize,
            "Cell is out of bounds! Cell index: {}, Map size: {}x{}",
            cell.0,
            width,
            height
        );

        let x = cell.0 as u32 % width;
        let y = cell.0 as u32 / width;

        OffsetCoordinate::from([x, y])
    }

    /// Converts an `OffsetCoordinate`  to a `Cell`. If the coordinate is out of bounds, an error is returned.
    ///
    /// # Arguments
    ///
    /// - `offset_coordinate` - The offset coordinate to convert.
    ///
    /// # Returns
    ///
    /// - `Result<Cell, String>`: The cell if the coordinate is valid, otherwise an error message.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    fn offset_to_cell(&self, offset_coordinate: OffsetCoordinate) -> Result<Cell, String> {
        self.normalize_offset(offset_coordinate)
            .map(|normalized_coordinate| {
                let [x, y] = normalized_coordinate.to_array();
                Cell((x + y * self.size().width as i32) as usize)
            })
    }

    /// Normalizes an offset coordinate to fit within the grid's bounds. If the coordinate is out of bounds, an error is returned.
    ///
    /// # Returns
    ///
    /// Returns a normalized `OffsetCoordinate` that fits within the grid's bounds or an error message.
    /// The normalized `OffsetCoordinate` should meet the conditions:
    /// - x ∈ [0, width)
    /// - y ∈ [0, height)
    ///
    /// If the coordinate is out of bounds, an error is returned.
    /// - If the grid is not wrapped in the X direction, and `offset_coordinate`'s `x` is out of bounds, i.e., not in the range `[0, width)`, the function will return an error.
    /// - If the grid is not wrapped in the Y direction, and `offset_coordinate`'s `y` is out of bounds, i.e., not in the range `[0, height)`, the function will return an error.
    #[must_use = "this `Result` may be an `Err` variant, which should be handled"]
    fn normalize_offset(
        &self,
        offset_coordinate: OffsetCoordinate,
    ) -> Result<OffsetCoordinate, String> {
        let mut x = offset_coordinate.0.x;
        let mut y = offset_coordinate.0.y;

        if self.wrap_x() {
            x = x.rem_euclid(self.width() as i32);
        }
        if self.wrap_y() {
            y = y.rem_euclid(self.height() as i32);
        }

        let offset_coordinate = OffsetCoordinate::new(x, y);

        if self.within_grid_bounds(offset_coordinate) {
            Ok(offset_coordinate)
        } else {
            Err(format!(
                "Offset coordinate out of bounds: {:?}",
                offset_coordinate
            ))
        }
    }

    /// Checks if the given `OffsetCoordinate` is within the grid's bounds.
    fn within_grid_bounds(&self, offset_coordinate: OffsetCoordinate) -> bool {
        offset_coordinate.0.x >= 0
            && offset_coordinate.0.x < self.width() as i32
            && offset_coordinate.0.y >= 0
            && offset_coordinate.0.y < self.height() as i32
    }

    /// Convert `GridCoordinateType` to a [`Cell`] in the grid.
    ///
    /// If the grid coordinate is out of bounds, it will return `None`.
    fn grid_coordinate_to_cell(&self, grid_coordinate: Self::GridCoordinateType) -> Option<Cell>;

    /// Computes the distance from `start` to `dest` in the grid.
    fn distance_to(&self, start: Cell, dest: Cell) -> i32;

    /// Returns the neighbor of `center` in the given `direction`.
    fn neighbor(self, center: Cell, direction: Direction) -> Option<Cell>;

    /// Returns an iterator over all grid cells that are at a distance of `distance` from `center`.
    ///
    /// # Arguments
    ///
    /// * `center` - The center cell.
    /// * `distance` - The distance from the center cell.
    ///
    /// # Notice
    ///
    /// Before calling:
    /// - For WarpX grids: `distance` should be ≤ `self.width() / 2`.
    /// - For WarpY grids: `distance` should be ≤ `self.height() / 2`.
    ///
    /// If you need distances beyond these limits (not recommended), filter results like this:
    /// ```rust, ignore
    /// let cells = grid.cells_at_distance(center, distance)
    /// .filter(|cell| {
    ///     grid.distance_to(center, *cell) == distance as i32
    /// }).collect::<Vec<_>>();
    /// ```
    #[must_use]
    fn cells_at_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Returns an iterator over all grid cells that are within a distance of `distance` from `center`.
    /// This includes the center cell itself.
    ///
    /// # Arguments
    ///
    /// - `center`: The center cell.
    /// - `distance`: The distance from the center cell.
    ///
    /// # Notice
    ///
    /// Before calling:
    /// - For WarpX grids: `distance` should be ≤ `self.width() / 2`.
    /// - For WarpY grids: `distance` should be ≤ `self.height() / 2`.
    ///
    /// If you need distances beyond these limits (not recommended), remove duplicate results like this:
    /// ```rust, ignore
    /// let cells = grid.cells_within_distance(center, distance)
    /// .collect::<HashSet<_>>();
    /// ```
    #[must_use]
    fn cells_within_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Determine the direction of `dest` relative to `start`.
    ///
    /// If `dest` is located to the north of `start`, the function returns `Some(Direction::North)`.
    /// If `dest` is equal to `start`, the function returns [`None`].
    fn estimate_direction(&self, start: Cell, dest: Cell) -> Option<Direction>;

    /// Create a rectangle region starting at `origin` with the specified `width` and `height` in the grid.
    fn rectangle_region(&self, origin: OffsetCoordinate, width: u32, height: u32) -> Rectangle
    where
        Self: Sized,
    {
        Rectangle::new(origin, width, height, self)
    }

    /// Create a rectangle region starting at `origin` and ending at `top_right_corner` in the grid.
    fn rectangle_region_from_corners(
        &self,
        origin: OffsetCoordinate,
        top_right_corner: OffsetCoordinate,
    ) -> Rectangle
    where
        Self: Sized,
    {
        Rectangle::from_corners(origin, top_right_corner, self)
    }
}

/// Represents the size of a grid or map with a specified width and height.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Size {
    /// The width of the grid or map.
    pub width: u32,
    /// The height of the grid or map.
    pub height: u32,
}

impl Size {
    /// Create a new `Size` with the specified width and height.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns how many cells are in the grid.
    ///
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

bitflags! {
    /// Bitflags representing how a grid/map wraps at its borders.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct WrapFlags: u8 {
        ///Enable horizontal wrapping (left/right edges connect)
        const WrapX = 0b0000_0001;
        /// Enable vertical wrapping (top/bottom edges connect)
        const WrapY = 0b0000_0010;
    }
}

/// Representing a cell or tile in a grid, which is identified by a unique index.
///
/// It is a wrapper around a `usize` index, which uniquely identifies the cell within the grid.
/// The index is used to access the cell in a flat representation of the grid, such as a 1D array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell(usize);

impl Cell {
    /// Creates a new `Cell` with the given index.
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// Returns the index of the cell in the grid.
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Trait for grids that can determine their world size type based on their size and provide a default size based on [WorldSizeType].
pub trait GridSize: Grid {
    /// Get world size type of the grid based on its size.
    ///
    fn world_size_type(&self) -> WorldSizeType {
        let width = self.width();
        let height = self.height();
        let area = width * height;

        // Get the threshold areas for each world size from default_size
        let duel_area = Self::default_size(WorldSizeType::Duel).area();
        let tiny_area = Self::default_size(WorldSizeType::Tiny).area();
        let small_area = Self::default_size(WorldSizeType::Small).area();
        let standard_area = Self::default_size(WorldSizeType::Standard).area();
        let large_area = Self::default_size(WorldSizeType::Large).area();
        let huge_area = Self::default_size(WorldSizeType::Huge).area();

        match area {
            // When area < duel_area, show warning
            area if area < duel_area => {
                eprintln!(
                    "The map size is too small. The provided dimensions are {}x{}, which gives an area of {}. The minimum area is {} in the original CIV5 game.",
                    width, height, area, duel_area
                );
                WorldSizeType::Duel
            }
            // Compare with each threshold
            area if area < tiny_area => WorldSizeType::Duel,
            area if area < small_area => WorldSizeType::Tiny,
            area if area < standard_area => WorldSizeType::Small,
            area if area < large_area => WorldSizeType::Standard,
            area if area < huge_area => WorldSizeType::Large,
            _ => WorldSizeType::Huge,
        }
    }

    /// Get the default size of the grid based on its world size type.
    /// This is used to initialize the grid with a default size.
    fn default_size(world_size_type: WorldSizeType) -> Size;
}

/// Defines standard world size type presets for game maps or environments.
///
/// Variants represent different scale levels from smallest to largest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorldSizeType {
    Duel,
    Tiny,
    Small,
    Standard,
    Large,
    Huge,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
/// Defines a rectangular region within a grid.
///
/// Provides functionality to retrieve all cells contained within the region.
pub struct Rectangle {
    /// The origin point in offset coordinates.
    ///
    /// # Grid Interpretation
    /// - Represents the south-west corner (bottom-left in visual terms)
    ///
    /// # Coordinate Constraints
    /// - x ∈ [0, grid_width)
    /// - y ∈ [0, grid_height)
    ///
    /// Where `grid_width` and `grid_height` are the dimensions of the containing grid.
    /// When you create a rectangle with [`Rectangle::new`] or [`Rectangle::from_corners`], the provided origin will be normalized to fit within these bounds.
    origin: OffsetCoordinate,
    /// The horizontal extent of the rectangle in cell units.
    ///
    /// # Requirements
    /// - Must be a positive integer (≥1)
    width: u32,
    /// The vertical extent of the rectangle in cell units.
    ///
    /// # Requirements
    /// - Must be a positive integer (≥1)
    height: u32,
}

impl Rectangle {
    /// Creates a new rectangle with the given origin, width, height, and grid.
    ///
    /// # Arguments
    ///
    /// - `origin`: The origin of the rectangle in offset coordinates.
    ///   This represents the bottom-left (south-west) corner of the rectangle in the grid.
    ///   It can be any valid offset coordinate,
    ///   we will process this origin to ensure its x is in the range `[0, grid_width - 1]` and y is in the range `[0, grid_height - 1]`.
    /// - `width`: The width of the rectangle in cells.
    /// - `height`: The height of the rectangle in cells.
    /// - `grid`: The grid is used to determine the map boundaries and wrapping behavior.
    ///
    /// # Panics
    ///
    /// This function will panic if the rectangle is not valid.
    pub fn new(origin: OffsetCoordinate, width: u32, height: u32, grid: &impl Grid) -> Self {
        // Debug-only validation
        debug_assert!(
            width > 0 && height > 0,
            "Rectangle dimensions must be positive (got {}x{})",
            width,
            height
        );
        debug_assert!(
            width <= grid.width() && height <= grid.height(),
            "Rectangle dimensions {}x{} exceed grid size {}x{}",
            width,
            height,
            grid.width(),
            grid.height()
        );

        let normalize_origin = grid
            .normalize_offset(origin)
            .unwrap_or_else(|_| panic!("Offset coordinate out of bounds: {:?}", origin));

        Self {
            origin: normalize_origin,
            width,
            height,
        }
    }

    /// Creates a new rectangle from the given origin and top-left corner.
    ///
    /// # Arguments
    ///
    /// - `origin`: The origin of the rectangle in offset coordinates.
    ///   This represents the bottom-left (south-west) corner of the rectangle in the grid.
    ///   It can be any valid offset coordinate,
    ///   we will process this origin to ensure its x is in the range `[0, grid_width - 1]` and y is in the range `[0, grid_height - 1]`.
    /// - `top_right_corner`: The top-right corner of the rectangle in offset coordinates.
    ///   This represents the top-right (north-east) corner of the rectangle in the grid.
    ///   It can be any valid offset coordinate.
    /// - `grid`: The grid is used to determine the map boundaries and wrapping behavior.
    ///
    /// # Panics
    /// This function will panic if the rectangle is not valid.
    pub fn from_corners(
        origin: OffsetCoordinate,
        top_right_corner: OffsetCoordinate,
        grid: &impl Grid,
    ) -> Self {
        let normalize_origin = grid
            .normalize_offset(origin)
            .unwrap_or_else(|_| panic!("Offset coordinate out of bounds: {:?}", origin));

        let [mut width, mut height] = (top_right_corner.0 - normalize_origin.0 + 1).to_array();

        if grid.wrap_x() {
            width = width.rem_euclid(grid.width() as i32);
        }
        if grid.wrap_y() {
            height = height.rem_euclid(grid.height() as i32);
        }

        debug_assert!(
            width > 0
                && width <= grid.width() as i32
                && height > 0
                && height <= grid.height() as i32,
            "The rectangle does not exist"
        );

        Self {
            origin,
            width: width as u32,
            height: height as u32,
        }
    }

    #[inline]
    pub fn origin(&self) -> OffsetCoordinate {
        self.origin
    }

    #[inline]
    pub fn west_x(&self) -> i32 {
        self.origin.0.x
    }

    #[inline]
    pub fn south_y(&self) -> i32 {
        self.origin.0.y
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns an iterator over all cells in current rectangle region of the grid.
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    pub fn all_cells<'a>(self, grid: &'a impl Grid) -> impl Iterator<Item = Cell> + 'a {
        (self.south_y()..self.south_y() + self.height as i32).flat_map(move |y| {
            (self.west_x()..self.west_x() + self.width as i32).map({
                move |x| {
                    let offset_coordinate = OffsetCoordinate::new(x, y);
                    grid.offset_to_cell(offset_coordinate).unwrap() // It's safe to unwrap because the offset is inside the current grid when the rectangle is valid.
                }
            })
        })
    }

    /// Checks if the given cell is inside the current rectangle.
    ///
    /// Returns `true` if the given cell is inside the current rectangle.
    pub fn contains(&self, cell: Cell, grid: &impl Grid) -> bool {
        let [mut x, mut y] = grid.cell_to_offset(cell).to_array();

        // We should consider the map is wrapped around horizontally.
        if x < self.west_x() {
            x += grid.width() as i32;
        }

        // We should consider the map is wrapped around vertically.
        if y < self.south_y() {
            y += grid.height() as i32;
        }

        x >= self.west_x()
            && x < self.west_x() + self.width as i32
            && y >= self.south_y()
            && y < self.south_y() + self.height as i32
    }

    /// Returns a new Rectangle that is a center crop of the original, scaled by the given factor.
    ///
    /// The resulting rectangle whose width and height are scaled by the given factor, and it is centered within the original rectangle.
    ///
    /// # Arguments
    ///
    /// * `scale`: The scaling factor (0.0 < scale <= 1.0).
    ///             1.0 returns the original rectangle, 0.5 returns a quarter of the area in the center of the original rectangle.
    /// * `grid`: The grid context required for the new Rectangle instance.
    ///
    /// # Panics
    ///
    /// Panics if `scale` is not in the range (0.0, 1.0].
    pub fn scaled_center_crop(&self, scale: f64, grid: &impl Grid) -> Rectangle {
        // --- Validation ---
        if scale <= 0.0 || scale > 1.0 {
            panic!(
                "Invalid scale factor: {}. Expected a value in range (0.0, 1.0].",
                scale
            );
        }

        // --- Calculation ---

        let original_width = self.width() as f64;
        let original_height = self.height() as f64;

        // Calculate target dimensions
        let target_width = original_width * scale;
        let target_height = original_height * scale;

        // Calculate padding to center the crop
        // floor() ensures we don't exceed bounds due to floating point errors
        let pad_x = ((original_width - target_width) / 2.0).floor() as u32;
        let pad_y = ((original_height - target_height) / 2.0).floor() as u32;

        // Derive final integer dimensions
        let final_width = self.width() - (pad_x * 2);
        let final_height = self.height() - (pad_y * 2);

        // Calculate bottom-left (or West/South) offset coordinates for the new rectangle
        let start_x = self.west_x() + pad_x as i32;
        let start_y = self.south_y() + pad_y as i32;

        Rectangle::new(
            OffsetCoordinate::new(start_x, start_y),
            final_width,
            final_height,
            grid,
        )
    }
}
