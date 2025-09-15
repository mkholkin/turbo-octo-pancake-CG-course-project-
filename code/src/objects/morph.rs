use crate::objects::Point;
use crate::objects::model3d::{InteractiveModel, Material, Model3D, Rotate, Scale, Triangle};
use crate::objects::triangle_mesh::TriangleMesh;
use crate::utils::morphing::{
    create_dcel_map, parametrize_mesh, relocate_vertices_on_mesh, triangulate_dcel, find_normals
};
use image::Rgb;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};

pub type VertexInterpolation = Box<dyn Fn(f32) -> Point>;
pub type NormalInterpolation = Box<dyn Fn(f32) -> Vector4<f32>>;

pub struct Morph {
    vertices: Vec<Point>,
    triangles: Vec<Triangle>,
    normals: Vec<Vector4<f32>>,
    vertex_interpolations: Vec<VertexInterpolation>,
    normals_interpolations: Vec<NormalInterpolation>,

    model_matrix: Matrix4<f32>,
}

impl Morph {
    pub fn new(mut obj_a: TriangleMesh, mut obj_b: TriangleMesh) -> Self {
        // 1. Параметризация исходных сеток
        parametrize_mesh(&mut obj_a);
        parametrize_mesh(&mut obj_b);

        //2. Пересечение исходной и целевой сеток
        println!("Построение DCEL MAP");
        let dcel = create_dcel_map(&obj_a, &obj_b);

        //3. Триангуляция граней пересеченной сетки
        println!("Триангуляция");
        let triangles = triangulate_dcel(&dcel);

        //4. Находим положения точек на исходной и целевой сетках
        println!("Барицентрические поиски");
        let src_vertices = relocate_vertices_on_mesh(&dcel.vertices, &obj_a);
        let dst_vertices = relocate_vertices_on_mesh(&dcel.vertices, &obj_b);

        let src_normals = find_normals(&src_vertices, &triangles, &obj_a);
        let dst_normals = find_normals(&dst_vertices, &triangles, &obj_b);

            //5. Строим интерполяции
            println!("Построение интерполяций");
        let vertex_interpolations: Vec<VertexInterpolation> = src_vertices
            .into_iter()
            .zip(dst_vertices.into_iter())
            .map(|(src_v, dst_v)| -> VertexInterpolation {
                Box::new(move |t: f32| Point::from((1. - t) * src_v.coords + t * dst_v.coords))
            })
            .collect();

        let normals_interpolations: Vec<NormalInterpolation> = src_normals
            .into_iter()
            .zip(dst_normals.into_iter())
            .map(|(src_n, dst_n)| -> NormalInterpolation {
                Box::new(move |t: f32| Vector4::from((1. - t) * src_n + t * dst_n))
            })
            .collect();

        //6. Строим вершины и нормали при t=0
        let vertices = vertex_interpolations.iter().map(|lerp| lerp(0.)).collect();
        let normals = normals_interpolations.iter().map(|lerp| lerp(0.)).collect();

        Morph {
            vertices,
            triangles,
            normals,
            vertex_interpolations,
            normals_interpolations,
            model_matrix: Matrix4::identity(),
        }
    }

    pub fn update(&mut self, t: f32) {
        // Рассчитать вершины
        for i in 0..self.vertices.len() {
            self.vertices[i] = self.vertex_interpolations[i](t);
        }

        // Рассчитать нормали
        for i in 0..self.normals.len() {
            self.normals[i] = self.model_matrix * self.normals_interpolations[i](t);
            self.normals[i].normalize_mut();
        }
    }
}

impl Model3D for Morph {
    fn triangles(&self) -> &Vec<Triangle> {
        &self.triangles
    }

    fn normals(&self) -> Vec<Vector4<f32>> {
        self.normals.clone()
    }

    fn vertices(&self) -> &Vec<Point> {
        &self.vertices
    }

    fn vertices_world(&self) -> Vec<Point> {
        // todo: iter
        self.vertices
            .iter()
            .map(|v| Point3::from_homogeneous(self.model_matrix * v.to_homogeneous()).unwrap())
            .collect()
    }

    fn material(&self) -> &Material {
        // TODO: Нормальный морфинг материала
        static MATERIAL: Material = Material {
            diffuse_reflectance_factor: 0.5,
            specular_reflectance_factor: 0.05,
            gloss: 1.,
            color: Rgb([208, 43, 43]),
            opacity: 0.1,
        };
        &MATERIAL
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

impl Rotate for Morph {
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

impl Scale for Morph {
    fn scale(&mut self, scaling: f32) {
        self.model_matrix = self.model_matrix * Matrix4::new_scaling(scaling);
    }
}

impl InteractiveModel for Morph {}
