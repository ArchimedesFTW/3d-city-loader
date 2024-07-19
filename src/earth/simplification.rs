use bevy::math::Vec2;

/// Simplifies a polygon by removing points that are not significant.
/// Uses Visvalingam–Whyatt to maintain the shape of the polygon.
pub fn simplify_polygon(polygon: Vec<Vec2>, threshold: f32) -> Vec<Vec2> {
    let mut polygon = polygon;

    // If the polygon is too small, return it as is
    if polygon.len() < 4 {
        return polygon;
    }

    // precompute areas spanned by all triangles
    let mut areas = Vec::with_capacity(polygon.len());
    let mut previous_point = polygon[polygon.len() - 1];
    let mut current_point = polygon[0];
    let mut next_point = polygon[1];
    for i in 0..polygon.len() {
        areas.push(triangle_area(previous_point, current_point, next_point));

        previous_point = current_point;
        current_point = next_point;
        next_point = polygon[(i + 2) % polygon.len()];
    }

    // Go over all triangles, remove the one with the smallest area
    while polygon.len() > 4 {
        let mut min_area = f32::MAX;
        let mut min_index = 0;
        for i in 0..areas.len() {
            if areas[i] < min_area {
                min_area = areas[i];
                min_index = i;
            }
        }

        // Remove the point with the smallest area if its under threshold
        if min_area > threshold {
            break;
        }

        // Remove the point
        polygon.remove(min_index);
        areas.remove(min_index);

        // Update the areas
        let previous_index = (min_index + polygon.len() - 1) % polygon.len();
        let next_index = min_index % polygon.len();
        areas[previous_index] = calculate_triangle_area_at_pos(&polygon, previous_index);
        areas[next_index] = calculate_triangle_area_at_pos(&polygon, next_index);
    }

    polygon
}

fn triangle_area(previous_point: Vec2, current_point: Vec2, next_point: Vec2) -> f32 {
    0.5 * (previous_point.x * (current_point.y - next_point.y)
        + current_point.x * (next_point.y - previous_point.y)
        + next_point.x * (previous_point.y - current_point.y))
        .abs()
}

fn calculate_triangle_area_at_pos(polygon: &Vec<Vec2>, i: usize) -> f32 {
    let previous_point = polygon[(i + polygon.len() - 1) % polygon.len()];
    let current_point = polygon[i];
    let next_point = polygon[(i + 1) % polygon.len()];
    let area = triangle_area(previous_point, current_point, next_point);
    area
}

// pub fn simplify_polyline(polyline: Vec<Vec2>) -> Vec<Vec2> {
// }
