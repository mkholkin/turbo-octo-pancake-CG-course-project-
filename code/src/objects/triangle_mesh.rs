use crate::objects::Point;
use crate::objects::model3d::{InteractiveModel, Material, Model3D, Rotate, Scale, Triangle};
use crate::utils::morphing::center_of_mass;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};

pub struct TriangleMesh {
    vertices: Vec<Point>,
    vertices_world: Vec<Point>, // Вершины умноженные на матрицу преобразования
    normals: Vec<Vector4<f32>>,
    triangles: Vec<Triangle>,
    material: Material,

    model_matrix: Matrix4<f32>,
}

impl Model3D for TriangleMesh {
    fn triangles(&self) -> &Vec<Triangle> {
        &self.triangles
    }

    fn normals(&self) -> &Vec<Vector4<f32>> {
        &self.normals
    }

    fn vertices(&self) -> &Vec<Point> {
        &self.vertices
    }

    fn vertices_world(&self) -> Vec<Point> {
        // TODO: iter
        self.vertices
            .iter()
            .map(|v| Point3::from_homogeneous(self.model_matrix * v.to_homogeneous()).unwrap())
            .collect()
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

    fn model_matrix(&self) -> &Matrix4<f32> {
        &self.model_matrix
    }
}

impl Rotate for TriangleMesh {
    fn rotate(&mut self, axis_angle_radians: (f32, f32, f32)) {
        let rotation_matrix = Matrix4::new_rotation(Vector3::new(
            axis_angle_radians.0,
            axis_angle_radians.1,
            axis_angle_radians.2,
        ));
        self.model_matrix = self.model_matrix * rotation_matrix;

        for n in &mut self.normals {
            *n = rotation_matrix * *n;
        }
    }
}

impl Scale for TriangleMesh {
    fn scale(&mut self, scaling: f32) {
        self.model_matrix = self.model_matrix * Matrix4::new_scaling(scaling);
    }
}

impl TriangleMesh {
    pub fn new() -> Self {
        TriangleMesh {
            vertices: Vec::new(),
            vertices_world: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
            material: Material::default(),
            model_matrix: Matrix4::identity(),
        }
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

        let mut mesh = TriangleMesh::new();
        let mut temp_normals: Vec<Vector4<f32>> = Vec::new();

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
                    let x = parts[1].parse::<f32>()?;
                    let y = parts[2].parse::<f32>()?;
                    let z = parts[3].parse::<f32>()?;
                    mesh.vertices.push(Point::new(x, y, z));
                }
                // Parse a normal line: `vn x y z`
                "vn" => {
                    let x = parts[1].parse::<f32>()?;
                    let y = parts[2].parse::<f32>()?;
                    let z = parts[3].parse::<f32>()?;
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

        mesh.vertices_world = mesh.vertices.clone();

        if !mesh.has_normals() {
            mesh.compute_normals();
        }
        
        mesh.centerify();
        
        Ok(mesh)
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<Point> {
        &mut self.vertices
    }
}

impl InteractiveModel for TriangleMesh {}
