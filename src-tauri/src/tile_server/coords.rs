use std::f64::consts::PI;
use tile_grid::Xyz;

pub struct BBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// Convert tile coordinates (z, x, y) to geographic bounding box
pub fn tile_to_bbox(z: u8, x: u32, y: u32) -> BBox {
    let tms = tile_grid::tms().lookup("WebMercatorQuad").unwrap();
    let tile = Xyz::new(x as u64, y as u64, z);
    let bounds = tms.bounds(&tile).unwrap();

    BBox {
        west: bounds.left,
        south: bounds.bottom,
        east: bounds.right,
        north: bounds.top,
    }
}

/// Convert lat/lng to pixel coordinates (0-4096) within a specific tile
pub fn lat_lng_to_tile_coords(lat: f64, lng: f64, z: u8, tile_x: u32, tile_y: u32) -> (f64, f64) {
    let bbox = tile_to_bbox(z, tile_x, tile_y);

    // X is linear in longitude
    let x = ((lng - bbox.west) / (bbox.east - bbox.west) * 4096.0).clamp(0.0, 4095.0);

    // Y requires Web Mercator projection (not linear in latitude!)
    // Convert latitude to Web Mercator Y
    let lat_to_merc_y = |lat_deg: f64| -> f64 {
        let lat_rad = lat_deg.to_radians();
        (1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / PI) / 2.0
    };

    let merc_y_point = lat_to_merc_y(lat);
    let merc_y_north = lat_to_merc_y(bbox.north);
    let merc_y_south = lat_to_merc_y(bbox.south);

    let y = ((merc_y_point - merc_y_north) / (merc_y_south - merc_y_north) * 4096.0).clamp(0.0, 4095.0);

    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Get which tile (XYZ) contains a given WGS84 lat/lng at a zoom level
    fn lat_lng_to_tile(lat: f64, lng: f64, zoom: u8) -> (u32, u32) {
        let n = (1u32 << zoom) as f64;

        // X tile calculation (simple longitude mapping)
        let x_tile = ((lng + 180.0) / 360.0 * n).floor() as u32;

        // Y tile calculation (Web Mercator projection)
        let lat_rad = lat.to_radians();
        let y_tile = ((1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / PI) / 2.0 * n).floor() as u32;

        let max_tile = (1u32 << zoom) - 1;
        (x_tile.min(max_tile), y_tile.min(max_tile))
    }

    #[test]
    fn test_tile_to_bbox() {
        // Tile 0,0,0 should be whole world
        let bbox = tile_to_bbox(0, 0, 0);
        assert!((bbox.west - (-180.0)).abs() < 0.01);
        assert!((bbox.east - 180.0).abs() < 0.01);
        // Web Mercator limits are approximately ±85.051129 degrees
        assert!((bbox.south - (-85.051129)).abs() < 0.01);
        assert!((bbox.north - 85.051129).abs() < 0.01);

        // Test zoom 1 tiles to understand coordinate system
        // In XYZ: y=0 is north pole, y=1 is south pole
        let bbox_north = tile_to_bbox(1, 0, 0);
        let bbox_south = tile_to_bbox(1, 0, 1);

        println!("Zoom 1, tile (0,0): south={:.4}, north={:.4}", bbox_north.south, bbox_north.north);
        println!("Zoom 1, tile (0,1): south={:.4}, north={:.4}", bbox_south.south, bbox_south.north);

        // In XYZ, tile y=0 should be the northern hemisphere
        assert!(bbox_north.north > bbox_north.south, "North tile should have north > south");
        assert!(bbox_north.south >= 0.0, "North tile (y=0) should be in northern hemisphere or at equator");

        // In XYZ, tile y=1 should be the southern hemisphere
        assert!(bbox_south.north <= 0.0, "South tile (y=1) should be in southern hemisphere or at equator");
    }

    #[test]
    fn test_lat_lng_to_tile_coords() {
        // Test exact coordinate mapping at different zoom levels

        // Zoom 0: whole world in one tile, 0°,0° is in center
        let (x, y) = lat_lng_to_tile_coords(0.0, 0.0, 0, 0, 0);
        assert_eq!(x, 2048.0, "Z=0: X should be exactly 2048 for longitude 0°");
        assert!((y - 2048.0).abs() < 0.001, "Z=0: Y should be near 2048 for latitude 0°, got {}", y);

        // Zoom 5: 0°,0° is at northwest corner of tile (16,16)
        // bbox: west=0, south=-11.178401873711781, east=11.25, north=0
        let (x, y) = lat_lng_to_tile_coords(0.0, 0.0, 5, 16, 16);
        assert_eq!(x, 0.0, "Z=5: 0° longitude is at west edge (x=0)");
        assert_eq!(y, 0.0, "Z=5: 0° latitude is at north edge (y=0)");

        // Zoom 10: 0°,0° is also at northwest corner of tile (512,512)
        let (x, y) = lat_lng_to_tile_coords(0.0, 0.0, 10, 512, 512);
        assert_eq!(x, 0.0, "Z=10: 0° longitude is at west edge (x=0)");
        assert_eq!(y, 0.0, "Z=10: 0° latitude is at north edge (y=0)");
    }

    #[test]
    fn test_costa_rica_point_at_multiple_zooms() {
        // San Isidro, Costa Rica: 10.0176473153°N, -84.0507658571°W
        let lat = 10.0176473153;
        let lng = -84.0507658571;

        // Zoom 1: should be in tile (0,0)
        let (tile_x, tile_y) = lat_lng_to_tile(lat, lng, 1);
        assert_eq!(tile_x, 0, "z=1: should be in tile x=0");
        assert_eq!(tile_y, 0, "z=1: should be in tile y=0");

        // With Web Mercator projection, the pixel coordinates will be different from linear interpolation
        let (pixel_x, pixel_y) = lat_lng_to_tile_coords(lat, lng, 1, tile_x, tile_y);
        assert!((pixel_x - 2183.0).abs() < 1.0, "z=1: pixel x should be near 2183, got {}", pixel_x);
        assert!((pixel_y - 3867.0).abs() < 1.0, "z=1: pixel y should be near 3867, got {}", pixel_y);

        // Zoom 5: should be in tile (8,15), pixel coords (2166, 430)
        let (tile_x, tile_y) = lat_lng_to_tile(lat, lng, 5);
        assert_eq!(tile_x, 8, "z=5: should be in tile x=8");
        assert_eq!(tile_y, 15, "z=5: should be in tile y=15");

        let (pixel_x, pixel_y) = lat_lng_to_tile_coords(lat, lng, 5, tile_x, tile_y);
        assert!((pixel_x - 2166.0).abs() < 1.0, "z=5: pixel x should be near 2166, got {}", pixel_x);
        assert!((pixel_y - 430.0).abs() < 1.0, "z=5: pixel y should be near 430, got {}", pixel_y);

        // Zoom 10: should be in tile (272,483), pixel coords (3777, 1470)
        let (tile_x, tile_y) = lat_lng_to_tile(lat, lng, 10);
        assert_eq!(tile_x, 272, "z=10: should be in tile x=272");
        assert_eq!(tile_y, 483, "z=10: should be in tile y=483");

        let (pixel_x, pixel_y) = lat_lng_to_tile_coords(lat, lng, 10, tile_x, tile_y);
        assert!((pixel_x - 3777.0).abs() < 1.0, "z=10: pixel x should be near 3777, got {}", pixel_x);
        assert!((pixel_y - 1471.0).abs() < 1.0, "z=10: pixel y should be near 1471, got {}", pixel_y);
    }
}
