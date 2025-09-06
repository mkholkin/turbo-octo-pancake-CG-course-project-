use nalgebra::{Matrix4, Point3, Vector3};

pub struct Camera {
    pub pos: Point3<f32>,
    look_at: Point3<f32>,
    up: Vector3<f32>,
    fov_radians: f32,
    aspect_ratio: f32,
    near_plane: f32,
    far_plane: f32,
    pub perspective_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,
    pub camera_matrix: Matrix4<f32>,
}

impl Camera {
    pub fn new(
        pos: Point3<f32>,
        look_at: Point3<f32>,
        up: Vector3<f32>,
        fov_radians: f32,
        aspect_ratio: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Self {
        let perspective_matrix =
            Matrix4::new_perspective(aspect_ratio, fov_radians, near_plane, far_plane);
        let view_matrix = Matrix4::look_at_rh(&pos, &look_at, &up);
        let camera_matrix = perspective_matrix * view_matrix;

        Camera {
            pos,
            look_at,
            up,
            fov_radians,
            aspect_ratio,
            near_plane,
            far_plane,
            perspective_matrix,
            view_matrix,
            camera_matrix,
        }
    }
}
