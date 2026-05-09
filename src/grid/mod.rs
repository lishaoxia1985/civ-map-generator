//! Grid system module providing coordinate systems.
//!
//! This module implements multiple coordinate systems to support different purposes.
//!
//! # Coordinate Systems Overview
//!
//! The module uses three complementary coordinate systems, each serving a specific purpose:
//!
//! ## Cell - Unique Position Identifier
//!
//! A [`Cell`] represents a unique position/tile in the grid, regardless of wrapping behavior.
//! It is the primary way to reference tiles because:
//! - Each cell has a unique linear index in a flattened 1D array
//! - In wrapped grids, a single cell may have multiple offset/grid coordinate representations
//! - Other coordinate systems cannot uniquely identify positions in wrapped grids
//!
//! ## Offset Coordinate - Column-Row System
//!
//! [`OffsetCoordinate`] describes the column and row of a tile in a grid.
//! It is used to tackle with the situation where the grid is wrapped.
//!
//! That picture below shows a unwrapped grid with offset coordinates.
//!
//! ```txt
//! Y ↑
//!   |
//!   |  (0,height-1)        (width-1,height-1)
//!   |  +-------------------+
//!   |  |                   |
//!   |  |    Grid Area      |
//!   |  |                   |
//!   |  +-------------------+
//!   |  (0,0)               (width-1,0)
//!   +--------------------------------→ X
//!   Origin (bottom-left corner)
//! ```
//!
//! The coordinate ranges depend on whether the grid wraps at boundaries:
//!
//! - **Non-wrapped grid**: `x ∈ [0, width)`, `y ∈ [0, height)`
//! - **Wrapped grid**:
//!   - Only Wrap x: x can be any value, y ∈ [0, height)
//!     - Example (x-wrapped): `(0, 0) ≡ (width, 0) ≡ (-width, 0) ≡ (2*width, 0)` is the same cell/tile
//!   - Only Wrap y: x ∈ [0, width), y can be any value
//!     - Example (y-wrapped): `(0, 0) ≡ (0, height) ≡ (0, -height) ≡ (0, 2*height)` is the same cell/tile
//!   - Wrap both x and y: x and y can be any value
//!     - Example (both x and y wrapped): `(0, 0) ≡ (width, height) ≡ (-width, -height) ≡ (2*width, 2*height)` is the same cell/tile
//!
//! In wrapped grids multiple offset coordinates can represent the same cell,
//! when we normalize an offset coordinate, i.e. wrap its x and y coordinates to the range `([0, width), [0, height))`,
//! it can be transformed into [`Cell`] uniquely.
//!
//! See the [`offset_coordinate`](self::offset_coordinate) module for detailed wrapping behavior.
//!
//! ## Grid Coordinate - Distance Calculation And Pixel Conversion
//!
//! Grid coordinates (hex or square) describe the distance between two tiles.
//!
//! Grid coordinates (hex or square) are used internally by the grid to calculate neighbors, distances, and other grid-related distance operations,
//! and to convert between grid coordinates and pixel coordinates.
//!
//! # Grid Shape
//!
//! This module only supports **rectangular** grids. Other shapes are not considered.

use direction::Direction;
use offset_coordinate::OffsetCoordinate;

use bitflags::bitflags;

pub mod direction;
pub mod hex_grid;
pub mod offset_coordinate;
pub mod square_grid;

/// Grid trait defining the interface for grid structures.
///
/// # Overview
///
/// This trait provides a unified interface for different grid types (hexagonal, square, etc.)
/// and defines core operations for coordinate manipulation, distance calculation, and spatial queries.
///
/// Grids use [`Cell`] to represent unique positions. Each grid implementation uses its specific
/// coordinate type:
/// - Hexagonal grids: [`Hex`](hex_grid::Hex) with axial coordinates (q, r)
/// - Square grids: [`Square`](square_grid::Square) with Cartesian coordinates (x, y)
///
/// These internal coordinates enable neighbor finding, distance calculations, and other
/// grid-related operations. All grids support conversion to/from [`OffsetCoordinate`] for
/// external interfaces.
///
/// # Wrapping Behavior
///
/// Grids can be configured with wrapping using [`WrapFlags`]:
/// - **WrapX**: Horizontal wrapping (left ↔ right edges connect)
/// - **WrapY**: Vertical wrapping (top ↔ bottom edges connect)
/// - **Both**: Toroidal topology (wraps in both directions)
/// - **Neither**: Standard rectangular grid with hard boundaries
///
/// Wrapping affects all operations including neighbor finding, distance calculation,
/// and pathfinding.
pub trait Grid {
    /// Internal coordinate type used by the grid.
    ///
    /// Used for neighbor calculations, distance measurements, and spatial operations.
    /// - Hex grids: [`Hex`](hex_grid::Hex)
    /// - Square grids: [`Square`](square_grid::Square)
    type GridCoordinateType;

    /// Direction array type for edges or corners.
    ///
    /// Should be `[Direction; N]` where N is the number of edges/corners:
    /// - Hex grids: `[Direction; 6]`
    /// - Square grids: `[Direction; 4]`
    type DirectionArrayType;

    /// Returns the array of edge directions for the grid.
    ///
    /// - Hex grids: 6 directions (edges of hexagon)
    /// - Square grids: 4 directions (edges of square)
    fn edge_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the array of corner directions for the grid.
    ///
    /// - Hex grids: 6 directions (vertices of hexagon)
    /// - Square grids: 4 directions (corners of square)
    fn corner_direction_array(&self) -> Self::DirectionArrayType;

    /// Returns the grid dimensions as a [`Size`] struct.
    fn size(&self) -> Size;

    /// Returns the grid width in cells (number of columns).
    fn width(&self) -> u32 {
        self.size().width
    }

    /// Returns the height of the grid in cells (number of rows).
    fn height(&self) -> u32 {
        self.size().height
    }

    /// Returns the flags that indicate how a grid/map wraps at its borders.
    ///
    /// See [`WrapFlags`] for details on wrapping behavior.
    fn wrap_flags(&self) -> WrapFlags;

    /// Returns if the grid is wrapped in the X direction (horizontal wrapping).
    ///
    /// When enabled, moving east from the rightmost column wraps to the leftmost column,
    /// and vice versa.
    fn wrap_x(&self) -> bool {
        self.wrap_flags().contains(WrapFlags::WrapX)
    }

    /// Returns if the grid is wrapped in the Y direction (vertical wrapping).
    ///
    /// When enabled, moving north from the top row wraps to the bottom row,
    /// and vice versa.
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
    /// - `offset_coordinate`: The offset coordinate to convert.
    ///   The offset coordinate's x and y should meet the condition below according to the grid's wrapping flags:
    ///    - **Non-wrapped grid**: `x ∈ [0, width)`, `y ∈ [0, height)`
    ///    - **Wrapped grid**:
    ///      - Only Wrap x: x can be any value, y ∈ [0, height)
    ///      - Only Wrap y: x ∈ [0, width), y can be any value
    ///      - Wrap both x and y: x and y can be any value
    fn offset_to_pixel(&self, offset_coordinate: OffsetCoordinate) -> [f32; 2];

    /// Returns the `OffsetCoordinate` which contains the given pixel coordinates.
    ///
    /// # Arguments
    ///
    /// - `pixel`: The pixel coordinate to convert.
    ///
    /// # Returns
    ///
    /// Returns the corresponding `OffsetCoordinate` in the grid's coordinate space.
    ///
    /// # Notes
    ///
    /// - **No Bounds Checking:** This function performs a direct mathematical conversion
    ///   and does not verify if the input pixel lies within the grid.
    ///   Consequently, the returned offset coordinate may be out of bounds.
    /// - **Intended Usage:** This method is typically used as a precursor to [`Self::offset_to_cell`].
    ///   Since that method handles bounds validation and returns an error for invalid coordinates,
    ///   explicit range checking is omitted here to avoid redundancy.
    fn pixel_to_offset(&self, pixel: [f32; 2]) -> OffsetCoordinate;

    /// Converts a `Cell` to an `OffsetCoordinate`.
    ///
    /// `OffsetCoordinate` is a normalized coordinate that fits within the grid's bounds.
    ///
    /// # Arguments
    ///
    /// - `cell`: The cell index to convert
    ///
    /// # Returns
    ///
    /// An `OffsetCoordinate` where:
    /// - x = cell_index % width (column position)
    /// - y = cell_index / width (row position)
    ///
    /// # Panics
    ///
    /// Panics in debug mode if the cell is out of bounds.
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

    /// Converts an `OffsetCoordinate` to a `Cell`. If the coordinate is out of bounds, an error is returned.
    ///
    /// # Arguments
    ///
    /// - `offset_coordinate`: The offset coordinate to convert.
    ///
    /// # Returns
    ///
    /// - `Result<Cell, String>`: The cell if the coordinate is valid, otherwise an error message.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let offset = OffsetCoordinate::new(3, 2);
    /// match grid.offset_to_cell(offset) {
    ///     Ok(cell) => println!("Cell index: {}", cell.index()),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    #[must_use = "this `Result` may be an `Err` variant, which should be handled"]
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

        if offset_coordinate.0.x >= 0
            && offset_coordinate.0.x < self.width() as i32
            && offset_coordinate.0.y >= 0
            && offset_coordinate.0.y < self.height() as i32
        {
            Ok(offset_coordinate)
        } else {
            Err(format!(
                "Offset coordinate out of bounds: {:?}",
                offset_coordinate
            ))
        }
    }

    /// Checks if the given `OffsetCoordinate` is within the grid bounds.
    ///
    /// # Notice
    ///
    /// If you only need to verify whether an offset coordinate is within the grid bounds, you can use this function.
    ///
    /// If you intend to normalize an offset coordinate while checking, you can call the [`Self::normalize_offset`] function directly and determine validity based on whether the result is `Ok`.
    ///
    /// If you intend to convert an offset coordinate to a cell while checking, you can call the [`Self::offset_to_cell`] function directly and determine validity based on whether the result is `Ok`.
    fn within_grid_bounds(&self, offset_coordinate: OffsetCoordinate) -> bool {
        let x = offset_coordinate.0.x;
        let y = offset_coordinate.0.y;
        match (self.wrap_x(), self.wrap_y()) {
            (true, true) => true,
            (true, false) => y >= 0 && y < self.height() as i32,
            (false, true) => x >= 0 && x < self.width() as i32,
            (false, false) => {
                x >= 0 && x < self.width() as i32 && y >= 0 && y < self.height() as i32
            }
        }
    }

    /// Convert `GridCoordinateType` to a [`Cell`] in the grid.
    ///
    /// If the grid coordinate is out of bounds, it will return `None`.
    fn grid_coordinate_to_cell(&self, grid_coordinate: Self::GridCoordinateType) -> Option<Cell>;

    /// Computes the distance from `start` to `dest` in the grid.
    ///
    /// For wrapped grids, the distance accounts for wrapping and returns the
    /// shortest path considering wrap-around.
    fn distance_to(&self, start: Cell, dest: Cell) -> i32;

    /// Returns the neighbor of `center` in the given `direction`.
    fn neighbor(self, center: Cell, direction: Direction) -> Option<Cell>;

    /// Returns an iterator over all grid cells that are at a distance of `distance` from `center`.
    ///
    /// # Arguments
    ///
    /// * `center`: The center cell.
    /// * `distance`: The distance from the center cell.
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
    ///     .filter(|cell| {
    ///     grid.distance_to(center, *cell) == distance as i32
    ///     }).collect::<Vec<_>>();
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Get all cells exactly 3 steps away from center
    /// let ring = grid.cells_at_distance(center, 3).collect::<Vec<_>>();
    /// ```
    #[must_use]
    fn cells_at_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Returns an iterator over all grid cells that are within a distance of `distance` from `center`.
    /// This includes the center cell itself.
    ///
    /// # Arguments
    ///
    /// - `center`: The center cell.
    /// - `distance`: The maximum distance from the center cell (inclusive).
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
    ///     .collect::<HashSet<_>>();
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Get all cells within 2 steps of center (includes center)
    /// let area = grid.cells_within_distance(center, 2).collect::<Vec<_>>();
    /// ```
    #[must_use]
    fn cells_within_distance(self, center: Cell, distance: u32) -> impl Iterator<Item = Cell>;

    /// Determine the direction of `dest` relative to `start`.
    ///
    /// Returns the primary compass direction from `start` to `dest`.
    /// The exact direction set depends on grid type (6 directions for hex, 4 for square).
    ///
    /// # Arguments
    ///
    /// - `start`: The starting cell
    /// - `dest`: The destination cell
    ///
    /// # Returns
    ///
    /// - `Some(Direction)` indicating the general direction from start to dest
    /// - [`None`] if `dest` equals `start` (no direction)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let to = grid.neighbor(from, Direction::North);
    /// if let Some(dir) = grid.estimate_direction(from, to) {
    ///     // dir would be `Direction::North` in this case
    ///     println!("Move {:?}", dir);
    /// }
    /// ```
    fn estimate_direction(&self, start: Cell, dest: Cell) -> Option<Direction>;

    /// Create a rectangle region starting at `origin` with the specified `width` and `height` in the grid.
    ///
    /// # Arguments
    ///
    /// - `origin`: The bottom-left (south-west) corner of the rectangle in offset coordinates
    /// - `width`: Width of the rectangle in cells
    /// - `height`: Height of the rectangle in cells
    /// - `grid`: The grid context for boundary checking
    ///
    /// # Returns
    ///
    /// A [`Rectangle`] object that can iterate over all contained cells
    fn rectangle_region(&self, origin: OffsetCoordinate, width: u32, height: u32) -> Rectangle
    where
        Self: Sized,
    {
        Rectangle::new(origin, width, height, self)
    }

    /// Create a rectangle region starting at `origin`(bottom-left corner) and ending at `top-right corner` in the grid.
    ///
    /// # Arguments
    ///
    /// - `origin`: The bottom-left (south-west) corner of the rectangle in offset coordinates
    /// - `top_right_corner`: The top-right (north-east) corner of the rectangle in offset coordinates
    /// - `grid`: The grid context for boundary checking and wrap handling
    ///
    /// # Returns
    ///
    /// A [`Rectangle`] object that can iterate over all contained cells
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

/// Represents the dimensions of a grid or map.
///
/// # Structure
///
/// Contains width and height as unsigned 32-bit integers, defining the grid's extent
/// in offset coordinate space.
///
/// # Visual Representation
///
/// ```txt
/// Y ↑
///   |
/// H |  +-------------------+
/// e |  |                   |
/// i |  |    Grid Area      |
/// g |  |    W × H cells    |
/// h |  |                   |
/// t |  +-------------------+
///   +--------------------------------→ X
///   0         Width
/// ```
///
/// # Examples
///
/// ```rust
/// use civ_map_generator::grid::Size;
///
/// let size = Size::new(10, 8);
/// assert_eq!(size.width, 10);
/// assert_eq!(size.height, 8);
/// assert_eq!(size.area(), 80); // 10 × 8 cells
/// ```
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Size {
    /// The width of the grid in cells (number of columns).
    pub width: u32,
    /// The height of the grid in cells (number of rows).
    pub height: u32,
}

impl Size {
    /// Create a new `Size` with the specified width and height.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns how many cells are in the grid.
    pub fn area(&self) -> u32 {
        self.width * self.height
    }
}

bitflags! {
    /// Bitflags representing how a grid/map wraps at its borders.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct WrapFlags: u8 {
        /// Enable horizontal wrapping (left/right edges connect).
        const WrapX = 0b0000_0001;
        /// Enable vertical wrapping (top/bottom edges connect).
        const WrapY = 0b0000_0010;
    }
}

/// Represents a unique position or tile in a grid, identified by a linear index.
///
/// # Overview
///
/// `Cell` is a wrapper around a `usize` index that provides a type-safe way to reference
/// individual tiles in a grid. The index corresponds to a position in a flattened 1D array
/// representation of the 2D grid.
///
/// # Index Calculation
///
/// For a grid with dimensions `width × height`:
/// ```txt
/// cell_index = x + y * width
/// ```
/// Where:
/// - `x ∈ [0, width)` is the column (offset x coordinate)
/// - `y ∈ [0, height)` is the row (offset y coordinate)
///
/// # Visual Layout
///
/// ```txt
/// Grid (4×3):
///
/// y=2:  [8]  [9]  [10] [11]
/// y=1:  [4]  [5]  [6]  [7]
/// y=0:  [0]  [1]  [2]  [3]
///       x=0  x=1  x=2  x=3
///
/// Cell indices increase left-to-right, bottom-to-top
/// ```
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

/// Defines a rectangular region within a grid.
///
/// Provides functionality to retrieve all cells contained within the region.
///
/// # Coordinate System
///
/// Rectangles are defined using offset coordinates with:
/// - **Origin**: The bottom-left (south-west) corner of the rectangle
/// - **Width**: Horizontal extent in cells (positive, ≥1)
/// - **Height**: Vertical extent in cells (positive, ≥1)
///
/// ```txt
/// Y ↑
///   |
/// H |  origin+width-1,height-1 ──────── top_right_corner
/// e |    (x+w-1, y+h-1)                    (x+w-1, y+h-1)
/// i |         +-------------------+
/// g |         |                   |
/// h |         |   Rectangle       |
/// t |         |    Area           |
///   |         |                   |
///   |         +-------------------+
///   |       origin (x, y)
///   +--------------------------------→ X
///   0         
/// ```
///
/// # Grid Interpretation
/// - Origin represents the south-west corner (bottom-left in visual terms)
/// - Rectangle extends positively in both x and y directions from origin
///
/// # Coordinate Constraints
/// - x ∈ [0, grid_width)
/// - y ∈ [0, grid_height)
///
/// Where `grid_width` and `grid_height` are the dimensions of the containing grid.
/// When you create a rectangle with [`Rectangle::new`] or [`Rectangle::from_corners`],
/// the provided origin will be normalized to fit within these bounds.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
    ///   1.0 returns the original rectangle, 0.5 returns a quarter of the area in the center of the original rectangle.
    /// * `grid`: The grid context required for the new Rectangle instance.
    ///
    /// # Panics
    ///
    /// In debug builds, this function panics if `scale` is not within the range `(0.0, 1.0]`.
    pub fn scaled_center_crop(&self, scale: f64, grid: &impl Grid) -> Rectangle {
        debug_assert!(
            scale > 0.0 && scale <= 1.0,
            "Invalid scale factor: {}. Expected a value in range (0.0, 1.0].",
            scale
        );

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
