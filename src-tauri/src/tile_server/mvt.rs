use mvt::{GeomEncoder, GeomType, Tile};
use pointy::Transform;

pub struct OccurrencePoint {
    pub core_id: i64,
    pub x: f64,
    pub y: f64,
    pub scientific_name: Option<String>,
}

/// Encode occurrence points as MVT protobuf bytes
pub fn encode_tile(points: Vec<OccurrencePoint>) -> Vec<u8> {
    let mut tile = Tile::new(4096);
    let mut layer = tile.create_layer("occurrences");

    for point in points {
        // Round coordinates to integers
        let x_rounded = point.x.round();
        let y_rounded = point.y.round();

        // Create geometry encoder
        let mut encoder = GeomEncoder::new(GeomType::Point, Transform::default());
        encoder.add_point(x_rounded, y_rounded).unwrap();
        let geom_data = encoder.encode().unwrap();

        // Create feature with geometry
        let mut feature = layer.into_feature(geom_data);

        // Add properties
        feature.add_tag_string("core_id", &point.core_id.to_string());
        if let Some(name) = point.scientific_name {
            feature.add_tag_string("scientificName", &name);
        }

        layer = feature.into_layer();
    }

    tile.add_layer(layer).unwrap();
    tile.to_bytes().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_empty_tile() {
        let tile = encode_tile(Vec::new());
        assert!(!tile.is_empty());
        // MVT protobuf should have minimal structure even when empty
    }

    #[test]
    fn test_encode_tile_with_points() {
        let points = vec![
            OccurrencePoint {
                core_id: 1,
                x: 2048.0,
                y: 2048.0,
                scientific_name: Some("Quercus alba".to_string()),
            },
        ];
        let tile = encode_tile(points);
        assert!(!tile.is_empty());
        assert!(tile.len() > 10); // Should have actual content
    }
}
