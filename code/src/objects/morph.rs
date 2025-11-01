use crate::objects::Point;
use crate::objects::model3d::{InteractiveModel, Material, Model3D, Rotate, Scale, Triangle};
use crate::objects::triangle_mesh::TriangleMesh;
use crate::utils::math::lerp;
use crate::utils::morphing::{
    create_supermesh, find_normals, parametrize_mesh, relocate_vertices_on_mesh,
};
use nalgebra::{Matrix4, Point3, Vector3, Vector4};

pub type Lerp<T> = Box<dyn Fn(f64) -> T>;
pub type VertexInterpolation = Lerp<Point>;
pub type NormalInterpolation = Lerp<Vector4<f64>>;
pub type MaterialInterpolation = Lerp<Material>;

pub struct Morph {
    vertices: Vec<Point>,
    vertices_world: Vec<Point>,
    triangles: Vec<Triangle>,
    normals: Vec<Vector4<f64>>,
    normals_world: Vec<Vector4<f64>>,
    material: Material,

    vertex_interpolations: Vec<VertexInterpolation>,
    normals_interpolations: Vec<NormalInterpolation>,
    material_interpolation: MaterialInterpolation,

    model_matrix: Matrix4<f64>,
}

impl Morph {
    pub fn new(source_object: TriangleMesh, target_object: TriangleMesh) -> Result<Self, String> {
        // 1. Параметризация исходных сеток
        let mut parametrized_source_mesh = source_object.clone();
        parametrize_mesh(&mut parametrized_source_mesh);

        let mut parametrized_target_mesh = target_object.clone();
        parametrize_mesh(&mut parametrized_target_mesh);

        // 2. Построение суперсетки
        let (vertices, triangles) =
            create_supermesh(&parametrized_source_mesh, &parametrized_target_mesh)?;

        // 3. Находим положения точек на исходной и целевой сетках
        let src_vertices = relocate_vertices_on_mesh(
            &vertices,
            &parametrized_source_mesh,
            source_object.vertices_world(),
        )?;
        let dst_vertices = relocate_vertices_on_mesh(
            &vertices,
            &parametrized_target_mesh,
            target_object.vertices_world(),
        )?;

        let src_normals = find_normals(
            &vertices,
            &triangles,
            &parametrized_source_mesh,
            source_object.normals(),
        )?;
        let dst_normals = find_normals(
            &vertices,
            &triangles,
            &parametrized_target_mesh,
            target_object.normals(),
        )?;

        // 4. Строим интерполяции
        let vertex_interpolations: Vec<VertexInterpolation> = src_vertices
            .into_iter()
            .zip(dst_vertices.into_iter())
            .map(|(src_v, dst_v)| -> VertexInterpolation {
                Box::new(move |t: f64| Point::from((1. - t) * src_v.coords + t * dst_v.coords))
            })
            .collect();

        let normals_interpolations: Vec<NormalInterpolation> = src_normals
            .into_iter()
            .zip(dst_normals.into_iter())
            .map(|(src_n, dst_n)| -> NormalInterpolation {
                Box::new(move |t: f64| lerp(src_n, dst_n, t))
            })
            .collect();

        let src_material = source_object.material().clone();
        let dst_material = target_object.material().clone();
        let material_interpolation: MaterialInterpolation =
            Box::new(move |t: f64| Material::lerp(&src_material, &dst_material, t));

        // 5. Строим интерполяции при t=0
        // 5.1 Строим вершины
        let vertices: Vec<Point> = vertex_interpolations.iter().map(|lerp| lerp(0.)).collect();
        let vertices_world = vertices.clone();

        // 5.2 Строим нормали
        let normals: Vec<Vector4<f64>> =
            normals_interpolations.iter().map(|lerp| lerp(0.)).collect();
        let normals_world = normals.clone();

        // 5.3 Строим материал
        let material = material_interpolation(0.);

        Ok(Morph {
            vertices,
            vertices_world,
            triangles,
            normals,
            normals_world,
            material,
            vertex_interpolations,
            normals_interpolations,
            material_interpolation,
            model_matrix: Matrix4::identity(),
        })
    }
}

impl Morph {
    fn update_vertices_world(&mut self) {
        for (vw, v) in self.vertices_world.iter_mut().zip(self.vertices.iter()) {
            *vw = Point3::from_homogeneous(self.model_matrix * v.to_homogeneous()).unwrap();
        }
    }

    fn update_normals_world(&mut self) {
        for (nw, n) in self.normals_world.iter_mut().zip(self.normals.iter()) {
            *nw = self.model_matrix * n;
            nw.normalize_mut();
        }
    }
}

impl Model3D for Morph {
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

    fn update(&mut self, t: f64) {
        // Рассчитать вершины
        for i in 0..self.vertices.len() {
            self.vertices[i] = self.vertex_interpolations[i](t);
        }

        // Рассчитать нормали
        for i in 0..self.normals.len() {
            self.normals[i] = self.normals_interpolations[i](t);
        }

        self.update_vertices_world();
        self.update_normals_world();

        // Рассчитать материал
        self.material = (self.material_interpolation)(t);
    }
}

impl Rotate for Morph {
    fn rotate(&mut self, axis_angle_radians: (f64, f64, f64)) {
        let rotation_matrix = Matrix4::new_rotation(Vector3::new(
            axis_angle_radians.0,
            axis_angle_radians.1,
            axis_angle_radians.2,
        ));
        self.model_matrix *= rotation_matrix;

        self.update_vertices_world();
        self.update_normals_world();
    }
}

impl Scale for Morph {
    fn scale(&mut self, scaling: f64) {
        self.model_matrix *= Matrix4::new_scaling(scaling);
        self.update_vertices_world();
        self.update_normals_world();
    }
}

impl InteractiveModel for Morph {
    fn reset_transformations(&mut self) {
        self.model_matrix = Matrix4::identity();
        self.update_vertices_world();
        self.update_normals_world();
    }
}
