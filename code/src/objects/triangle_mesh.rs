use crate::objects::Point;
use crate::objects::model3d::{InteractiveModel, Material, Model3D, Rotate, Scale, Triangle};
use crate::utils::dcel::DCEL;
use crate::utils::morphing::{center_of_mass, triangulate_dcel};
use image::Rgb;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
#[derive(Clone)]
pub struct TriangleMesh {
    pub vertices: Vec<Point>,
    pub vertices_world: Vec<Point>, // Вершины умноженные на матрицу преобразования
    normals: Vec<Vector4<f64>>,
    normals_world: Vec<Vector4<f64>>, // Нормали умноженные на матрицу преобразования
    triangles: Vec<Triangle>,
    pub material: Material,

    pub model_matrix: Matrix4<f64>,
}

impl Default for TriangleMesh {
    fn default() -> Self {
        Self {
            vertices: Vec::default(),
            vertices_world: Vec::default(),
            normals: Vec::default(),
            normals_world: Vec::default(),
            triangles: Vec::default(),
            material: Material::default(),
            model_matrix: Matrix4::identity(),
        }
    }
}

impl TriangleMesh {
    pub fn update_normals_world(&mut self) {
        for i in 0..self.normals.len() {
            self.normals_world[i] = (self.model_matrix * self.normals[i]).normalize();
        }
    }

    pub fn update_vertices_world(&mut self) {
        for i in 0..self.vertices.len() {
            if let Some(point) = Point3::from_homogeneous(self.model_matrix * self.vertices[i].to_homogeneous()) {
                self.vertices_world[i] = point;
            }
        }
    }

    pub fn reset_transformations(&mut self) {
        self.model_matrix = Matrix4::identity();
        self.update_vertices_world();
        self.update_normals_world();
    }
}

impl Model3D for TriangleMesh {
    fn triangles(&self) -> &Vec<Triangle> {
        &self.triangles
    }

    fn normals(&self) -> &Vec<Vector4<f64>> {
        &self.normals_world
    }

    fn vertices(&self) -> &Vec<Point> {
        &self.vertices
    }

    fn vertices_world(&self) -> &Vec<Point> {
        &self.vertices_world
    }

    fn material(&self) -> &Material {
        &self.material
    }

    fn has_normals(&self) -> bool {
        !self.normals.is_empty()
    }

    fn compute_normals(&mut self) {
        todo!()
    }

    fn model_matrix(&self) -> &Matrix4<f64> {
        &self.model_matrix
    }
}

impl Rotate for TriangleMesh {
    fn rotate(&mut self, axis_angle_radians: (f64, f64, f64)) {
        let rotation_matrix = Matrix4::new_rotation(Vector3::new(
            axis_angle_radians.0,
            axis_angle_radians.1,
            axis_angle_radians.2,
        ));
        self.model_matrix = self.model_matrix * rotation_matrix;

        self.update_normals_world();
        self.update_vertices_world();
    }
}

impl Scale for TriangleMesh {
    fn scale(&mut self, scaling: f64) {
        self.model_matrix = self.model_matrix * Matrix4::new_scaling(scaling);
        self.update_vertices_world()
    }
}

impl TriangleMesh {
    fn centerify(&mut self) {
        let center = center_of_mass(self);
        for v in &mut self.vertices {
            *v -= center;
        }
    }

    /// Helper function for parsing faces
    /// Parses a single component of a face line (`v`, `v/vt`, `v//vn`, `v/vt/vn`)
    /// and validates the vertex and normal indices.
    ///
    /// This function ignores the texture index (`vt`) and returns the normal index as an `Option<usize>`,
    /// which will be `None` if a normal index is not present in the face component.
    fn parse_and_validate_face_part(
        part: &str,
        total_vertices: usize,
        total_normals: usize,
        line_number: usize,
    ) -> Result<(usize, Option<usize>), Box<dyn Error>> {
        let indices: Vec<&str> = part.split('/').collect();

        if indices.is_empty() {
            return Err(format!("Invalid face format on line {}", line_number).into());
        }

        // The first part is always the vertex index.
        let v_idx = indices[0].parse::<usize>()? - 1;
        if v_idx >= total_vertices {
            return Err(
                format!("Invalid vertex index {} on line {}", v_idx + 1, line_number).into(),
            );
        }

        // The normal index is the third part if it exists and is not empty.
        let n_idx = match indices.get(2) {
            Some(n_str) if !n_str.is_empty() => {
                let n_idx_parsed = n_str.parse::<usize>()? - 1;
                if n_idx_parsed >= total_normals {
                    return Err(format!(
                        "Invalid normal index {} on line {}",
                        n_idx_parsed + 1,
                        line_number
                    )
                    .into());
                }
                Some(n_idx_parsed)
            }
            _ => None,
        };

        Ok((v_idx, n_idx))
    }

    /// Read from .obj file
    pub fn from_obj(path: &str) -> Result<Self, Box<dyn Error>> {
        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);

        let mut mesh = TriangleMesh::default();
        let mut temp_normals: Vec<Vector4<f64>> = Vec::new();

        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();

            // Skip empty lines
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                // Parse vertex line: `v x y z`
                "v" => {
                    let x = parts[1].parse::<f64>()?;
                    let y = parts[2].parse::<f64>()?;
                    let z = parts[3].parse::<f64>()?;
                    mesh.vertices.push(Point::new(x, y, z));
                }
                // Parse a normal line: `vn x y z`
                "vn" => {
                    let x = parts[1].parse::<f64>()?;
                    let y = parts[2].parse::<f64>()?;
                    let z = parts[3].parse::<f64>()?;
                    temp_normals.push(Vector4::new(x, y, z, 0.).normalize());
                }
                // Parse a face line: `f v1//vn1 v2//vn2 v3//vn3`
                "f" => {
                    if parts.len() != 4 {
                        return Err("Face must have at 3 vertices (Quads are not supported)".into());
                    }

                    let total_vertices = mesh.vertices.len();
                    let total_normals = temp_normals.len();

                    // Parse the first three vertices and normals for the first triangle.
                    let (v1_idx, n1_opt) = Self::parse_and_validate_face_part(
                        parts[1],
                        total_vertices,
                        total_normals,
                        i + 1,
                    )?;
                    let (v2_idx, n2_opt) = Self::parse_and_validate_face_part(
                        parts[2],
                        total_vertices,
                        total_normals,
                        i + 1,
                    )?;
                    let (v3_idx, n3_opt) = Self::parse_and_validate_face_part(
                        parts[3],
                        total_vertices,
                        total_normals,
                        i + 1,
                    )?;

                    // Push the first triangle's vertex indices.
                    mesh.triangles.push((v1_idx, v2_idx, v3_idx));

                    // Push the normal vector it exists.
                    let n_idx = n1_opt.or(n2_opt).or(n3_opt);
                    if let Some(n_idx) = n_idx {
                        mesh.normals.push(temp_normals[n_idx]);
                    }
                }
                // Ignore other lines like `g` (group) or comments (`#`)
                _ => {}
            }
        }

        if !mesh.has_normals() {
            mesh.compute_normals();
        }

        mesh.centerify();
        mesh.vertices_world = mesh.vertices.clone();
        mesh.normals_world = mesh.normals.clone();

        Ok(mesh)
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<Point> {
        &mut self.vertices
    }
    pub fn vertices_world_mut(&mut self) -> &mut Vec<Point> {
        &mut self.vertices_world
    }
}

impl From<DCEL> for TriangleMesh {
    fn from(dcel: DCEL) -> Self {
        let mut mesh = Self::default();

        mesh.triangles = triangulate_dcel(&dcel);
        mesh.vertices = dcel.vertices;
        mesh.vertices_world = mesh.vertices.clone();
        mesh.material.color = Rgb([0, 255, 0]);

        mesh
    }
}

impl InteractiveModel for TriangleMesh {
    fn reset_transformations(&mut self) {
        self.model_matrix = Matrix4::identity();
        self.update_vertices_world();
        self.update_normals_world();
    }
}
