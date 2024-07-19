
use bevy::math::{Mat3, Vec2, Vec3, Vec4Swizzles};
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::transform::components::Transform;
use wasm_bindgen::prelude::*;

use earcutr::earcut;

use geo::{coord, CoordsIter, LineString, Polygon};

use std::iter::repeat;

pub struct MeshBuilder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    uvs: Vec<Vec2>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    /// Creates a new mesh builder with no vertices
    pub fn new() -> Self {
        MeshBuilder {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Adds a new vertex to the mesh and returns its index.
    pub fn add_vertex(
        &mut self,
        position: Vec3,
        normal: Vec3,
        uv: Vec2,
    ) -> u32 {
        self.positions.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);

        assert_eq!(self.normals.len(), self.positions.len());
        assert_eq!(self.uvs.len(), self.positions.len());

        self.positions.len() as u32 - 1
    }

    /// Adds a quad to the mesh. `coords` should be in counterclockwise order
    /// of the quad, assuming a right handed system.
    pub fn add_quad(
        &mut self,
        positions: [Vec3; 4],
        uvs: [Vec2; 4],
    ) {
        let bottom_line = positions[1] - positions[0];
        let up_line = positions[2] - positions[1];
        let normal = up_line.cross(bottom_line).normalize();
        let a = self.add_vertex(positions[0], normal, uvs[0]);
        let b = self.add_vertex(positions[1], normal, uvs[1]);
        let c = self.add_vertex(positions[2], normal, uvs[2]);
        let d = self.add_vertex(positions[3], normal, uvs[3]);

        self.indices.extend([ c, b, a ]);
        self.indices.extend([ a, c, d ]);
    }

    /// Adds the faces of a vertically-oriented prism to the mesh with the
    /// given `polygon` as the base, and with `y1` as the bottom and `y2` as
    /// the top.
    pub fn add_polygon_xz(
        &mut self,
        polygon: &Polygon,
        y: f32,
        uv: Vec2,
    ) {
        let coords_flat = polygon.exterior_coords_iter()
            .flat_map(|coord| [coord.x, coord.y])
            .collect::<Vec<_>>();
        let triangulation = earcut(&coords_flat, &[], 2).unwrap_throw();

        let index_offset = self.positions.len();
        let vertices = polygon.exterior().coords_count();

        // add vertices
        self.positions.extend(
            polygon.exterior_coords_iter()
                .map(|coord| Vec3::new(coord.x as f32, y, coord.y as f32)),
        );
        self.normals.extend(repeat(Vec3::Y).take(vertices));
        self.uvs.extend(repeat(uv).take(vertices));

        assert_eq!(self.normals.len(), self.positions.len());
        assert_eq!(self.uvs.len(), self.positions.len());

        // add bottom face indices
        self.indices.extend(
            triangulation.iter()
                .map(|&index| (index_offset + index) as u32),
        );
    }

    pub fn get_triangle_from_earcuttr(&self, polygon: &Polygon) -> Vec<[Vec3; 3]> {
        let coords_flat = polygon.exterior_coords_iter()
            .flat_map(|coord| [coord.x, coord.y])
            .collect::<Vec<_>>();
        let triangulation = earcut(&coords_flat, &[], 2).unwrap_throw();

        triangulation.chunks(3)
            .map(|chunk| [
                Vec3::new(
                    polygon.exterior_coords_iter().nth(chunk[0]).unwrap_throw().x as f32,
                    0.,
                    polygon.exterior_coords_iter().nth(chunk[0]).unwrap_throw().y as f32,
                ),
                Vec3::new(
                    polygon.exterior_coords_iter().nth(chunk[1]).unwrap_throw().x as f32,
                    0.,
                    polygon.exterior_coords_iter().nth(chunk[1]).unwrap_throw().y as f32,
                ),
                Vec3::new(
                    polygon.exterior_coords_iter().nth(chunk[2]).unwrap_throw().x as f32,
                    0.,
                    polygon.exterior_coords_iter().nth(chunk[2]).unwrap_throw().y as f32,
                ),
            ])
            .collect()
    }

    /// Get triangles from the mesh
    pub fn get_triangles(&self) -> Vec<[Vec3; 3]> {
        self.indices.chunks(3)
            .map(|chunk| [
                self.positions[chunk[0] as usize],
                self.positions[chunk[1] as usize],
                self.positions[chunk[2] as usize],
            ])
            .collect()
    }
    
    /// Generates a Bevy mesh given the 2D path (of points) and extrude amount.
    /// `path_2d` is assumed to be in counter-clockwise order.
    pub fn add_prism_from_path(
        &mut self,
        path_2d: &Vec<Vec2>,
        extrude_amount: f32,
        uv: Vec2,
    ) {
        // Floor and ceiling heights
        let y1 = 0.;
        let y2 = extrude_amount;

        let polygon = Polygon::new(
            LineString::new(
                path_2d
                    .iter()
                    .map(|p| coord! {x: p.x as f64, y: p.y as f64})
                    .collect::<Vec<_>>(),
            ),
            Vec::new(),
        );

        // Ceiling
        self.add_polygon_xz(&polygon, y2, uv);

        // For every line along the polygon base, add a face
        for line in polygon.exterior().lines() {
            let corner1 = Vec3::new(line.end.x as f32, y1, line.end.y as f32);
            let corner2 = Vec3::new(line.start.x as f32, y1, line.start.y as f32);
            let corner3 = Vec3::new(line.start.x as f32, y2, line.start.y as f32);
            let corner4 = Vec3::new(line.end.x as f32, y2, line.end.y as f32);

            self.add_quad([corner1, corner4, corner3, corner2], [uv, uv, uv, uv]);
        }
    }

    /// Adds an entire already-built mesh to the final mesh.
    ///
    /// # Preconditions
    /// It is assumed that `mesh` uses indices, a position, a normal and
    /// uv/texture coordinates.
    pub fn add_mesh(&mut self, mesh: &Mesh, transform: Transform) {
        let index_offset = self.positions.len() as u32;

        let attribute = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
        if let Some(VertexAttributeValues::Float32x3(positions)) = attribute {
            for &position in positions {
                let point = transform.transform_point(Vec3::from_array(position));
                self.positions.push(point);
            }
        } else {
            panic!("Expected (f32, f32, f32) positions in the mesh");
        }

        // NOTE: some funky stuff has to be done here, see mikktspace.com,
        // bevy's `mesh_normal_local_to_world()` in `mesh_functions.wgsl`,
        // gist.github.com/DGriffin91/e63e5f7a90b633250c2cf4bf8fd61ef8
        let mat = transform.compute_matrix().inverse().transpose();
        let mat = Mat3 {
            x_axis: mat.x_axis.xyz(),
            y_axis: mat.y_axis.xyz(),
            z_axis: mat.z_axis.xyz(),
        };
        let attribute = mesh.attribute(Mesh::ATTRIBUTE_NORMAL);
        if let Some(VertexAttributeValues::Float32x3(normals)) = attribute {
            for &normal in normals {
                let vector = mat.mul_vec3(Vec3::from_array(normal))
                    .normalize_or_zero().into();
                self.normals.push(vector);
            }
        } else {
            panic!("Expected (f32, f32, f32) normals in the mesh");
        }

        let attribute = mesh.attribute(Mesh::ATTRIBUTE_UV_0);
        if let Some(VertexAttributeValues::Float32x2(uvs)) = attribute {
            for &uv in uvs {
                self.uvs.push(Vec2::from_array(uv));
            }
        } else {
            panic!("Expected (f32, f32) uv coordinates in the mesh");
        }

        assert_eq!(self.normals.len(), self.positions.len());
        assert_eq!(self.uvs.len(), self.positions.len());

        let indices = mesh.indices().expect("Expected indices in the mesh");
        for index in indices.iter() {
            self.indices.push(index_offset + index as u32);
        }
    }

    /// Turns the data into a bevy `Mesh`.
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));

        mesh
    }
}
